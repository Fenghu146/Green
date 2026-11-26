//! Reading a PDF back: pages, bytes per font, pictures, compression, coverage.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};
use sha2::{Digest, Sha256};

mod painted;

/// A page's mean bytes above which the export is called heavy.
const MEAN_PAGE_BYTES: f64 = 500.0 * 1024.0;
/// An embedded font above which the export is called heavy.
const FONT_BYTES: u64 = 3 * 1024 * 1024;
/// A share of the file taken by pictures above which the export is called heavy.
const IMAGE_SHARE: f64 = 0.70;
/// A share of a page covered by one picture above which the export is called heavy.
const COVERAGE: f64 = 0.40;

/// What an audit found in a PDF (design §7).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Audit {
    /// Page count.
    pub pages: usize,
    /// The file's bytes on disk.
    pub bytes: u64,
    /// Embedded fonts, heaviest first.
    pub fonts: Vec<Font>,
    /// Image XObjects, however many pages place them.
    pub image_refs: usize,
    /// The same, by content hash.
    pub images: usize,
    /// The bytes image streams take.
    pub image_bytes: u64,
    /// The filters those streams use.
    pub image_filters: BTreeSet<String>,
    /// Whether every page's content stream is FlateDecode.
    pub compressed: bool,
    /// The page with the widest picture, 1 based.
    pub widest_page: Option<u32>,
    /// That picture's share of the page's area, 0 to 1.
    pub widest_coverage: f64,
}

/// An embedded font: its name, and the bytes it takes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Font {
    /// The name without the subset prefix or encoding suffix.
    pub name: String,
    /// The embedded program's bytes.
    pub bytes: u64,
}

/// What design §7 calls too heavy.
#[derive(Clone, Debug, PartialEq)]
pub enum Warning {
    /// The pages average more than 500 KB.
    MeanPage {
        /// Bytes per page.
        bytes: u64,
    },
    /// One font carries more than 3 MB.
    Font {
        /// Its name.
        name: String,
        /// Its bytes.
        bytes: u64,
    },
    /// Pictures take more than 70% of the file.
    Images {
        /// Their share, 0 to 1.
        share: f64,
    },
    /// One picture covers more than 40% of its page.
    Coverage {
        /// The page, 1 based.
        page: u32,
        /// Its share, 0 to 1.
        share: f64,
    },
    /// Characters none of the bundled fonts draws: Typst would print tofu.
    MissingGlyphs {
        /// Each missing character, with where it first appears.
        found: Vec<String>,
    },
}

impl Warning {
    /// For the log and the status bar.
    pub fn message(&self) -> String {
        match self {
            Self::MeanPage { bytes } => {
                format!("export pdf: {bytes} bytes per page is over 500 KB")
            }
            Self::Font { name, bytes } => {
                format!("export pdf: font {name} is {bytes} bytes, over 3 MB")
            }
            Self::Images { share } => {
                format!(
                    "export pdf: pictures take {:.1}% of the file, over 70%",
                    share * 100.0
                )
            }
            Self::Coverage { page, share } => format!(
                "export pdf: page {page} has a picture over {:.1}% of its area, over 40%",
                share * 100.0
            ),
            Self::MissingGlyphs { found } => format!(
                "export: {} characters are in none of the bundled fonts: {}",
                found.len(),
                found.join(", ")
            ),
        }
    }
}

impl Audit {
    /// The design §7 line, one per export.
    pub fn line(&self) -> String {
        let fonts: Vec<String> = self
            .fonts
            .iter()
            .map(|font| format!("{{\"name\":\"{}\",\"bytes\":{}}}", font.name, font.bytes))
            .collect();
        let images = match self.images {
            0 => "images=0/0 (dup:0)".to_string(),
            _ => {
                let filters: Vec<&str> = self.image_filters.iter().map(String::as_str).collect();
                format!(
                    "images={}/{} (dup:{}, filters=[{}])",
                    self.image_refs,
                    self.images,
                    self.image_refs - self.images,
                    filters.join(",")
                )
            }
        };
        format!(
            "export pdf: pages={} bytes={} fonts=[{}] {} compressed={} max_image_coverage={:.1}%",
            self.pages,
            self.bytes,
            fonts.join(","),
            images,
            self.compressed,
            self.widest_coverage * 100.0
        )
    }

    /// Every threshold the audit crosses.
    pub fn warnings(&self) -> Vec<Warning> {
        let mut warnings = Vec::new();
        let mean = match self.pages {
            0 => 0.0,
            pages => self.bytes as f64 / pages as f64,
        };
        if mean > MEAN_PAGE_BYTES {
            warnings.push(Warning::MeanPage { bytes: mean as u64 });
        }
        for font in self.fonts.iter().filter(|font| font.bytes > FONT_BYTES) {
            warnings.push(Warning::Font {
                name: font.name.clone(),
                bytes: font.bytes,
            });
        }
        let share = match self.bytes {
            0 => 0.0,
            bytes => self.image_bytes as f64 / bytes as f64,
        };
        if share > IMAGE_SHARE {
            warnings.push(Warning::Images { share });
        }
        if self.widest_coverage > COVERAGE {
            warnings.push(Warning::Coverage {
                page: self.widest_page.unwrap_or(0),
                share: self.widest_coverage,
            });
        }
        warnings
    }
}

/// Audit the PDF at `path`.
pub fn read(path: &Path) -> Result<Audit> {
    let bytes = std::fs::metadata(path)
        .with_context(|| format!("measuring {}", path.display()))?
        .len();
    let doc = Document::load(path).with_context(|| format!("reading {}", path.display()))?;
    let pages = doc.get_pages();
    let (image_refs, images, image_bytes, image_filters) = pictures(&doc);
    let (widest_page, widest_coverage) = painted::widest(&doc, &pages);
    Ok(Audit {
        pages: pages.len(),
        bytes,
        fonts: fonts(&doc),
        image_refs,
        images,
        image_bytes,
        image_filters,
        compressed: compressed(&doc, &pages),
        widest_page,
        widest_coverage,
    })
}

/// Each font's embedded bytes, heaviest first.
fn fonts(doc: &Document) -> Vec<Font> {
    let mut sizes: HashMap<ObjectId, u64> = HashMap::new();
    let mut owners: HashMap<String, HashSet<ObjectId>> = HashMap::new();
    for object in doc.objects.values() {
        let Ok(dict) = object.as_dict() else {
            continue;
        };
        let Some(base) = dict
            .get(b"BaseFont")
            .ok()
            .and_then(|object| object.as_name().ok())
        else {
            continue;
        };
        let Some((key, descriptor)) = descriptor(doc, dict) else {
            continue;
        };
        sizes
            .entry(key)
            .or_insert_with(|| embedded(doc, descriptor));
        owners.entry(font_name(base)).or_default().insert(key);
    }
    let mut fonts: Vec<Font> = owners
        .into_iter()
        .map(|(name, keys)| Font {
            name,
            bytes: keys
                .iter()
                .map(|key| sizes.get(key).copied().unwrap_or(0))
                .sum(),
        })
        .collect();
    fonts.sort_by(|left, right| {
        right
            .bytes
            .cmp(&left.bytes)
            .then(left.name.cmp(&right.name))
    });
    fonts
}

/// A font's descriptor, by the object it lives in.
fn descriptor<'a>(doc: &'a Document, font: &'a Dictionary) -> Option<(ObjectId, &'a Dictionary)> {
    match font.get(b"FontDescriptor").ok()? {
        Object::Reference(id) => Some((*id, doc.get_dictionary(*id).ok()?)),
        Object::Dictionary(dict) => Some(((0, 0), dict)),
        _ => None,
    }
}

/// The bytes of the font programs a descriptor embeds.
fn embedded(doc: &Document, descriptor: &Dictionary) -> u64 {
    [b"FontFile".as_slice(), b"FontFile2", b"FontFile3"]
        .iter()
        .filter_map(|key| {
            descriptor
                .get(key)
                .ok()
                .and_then(|object| stream(doc, object))
        })
        .map(|stream| stream.content.len() as u64)
        .sum()
}

fn stream<'a>(doc: &'a Document, object: &'a Object) -> Option<&'a Stream> {
    match object {
        Object::Reference(id) => doc.get_object(*id).ok()?.as_stream().ok(),
        Object::Stream(stream) => Some(stream),
        _ => None,
    }
}

/// `ABCDEF+Name-Identity-H` as `Name`.
fn font_name(base: &[u8]) -> String {
    let raw = String::from_utf8_lossy(base).into_owned();
    let name = match raw.split_once('+') {
        Some((subset, rest))
            if subset.len() == 6 && subset.chars().all(|ch| ch.is_ascii_uppercase()) =>
        {
            rest
        }
        _ => raw.as_str(),
    };
    name.strip_suffix("-Identity-H")
        .or_else(|| name.strip_suffix("-Identity-V"))
        .unwrap_or(name)
        .to_string()
}

/// Image references, distinct images by content hash, their bytes and filters.
fn pictures(doc: &Document) -> (usize, usize, u64, BTreeSet<String>) {
    let (mut refs, mut bytes, mut filters) = (0, 0, BTreeSet::new());
    let mut distinct = HashSet::new();
    for object in doc.objects.values() {
        let Ok(stream) = object.as_stream() else {
            continue;
        };
        if stream.dict.get(b"Subtype").and_then(Object::as_name).ok() != Some(b"Image".as_slice()) {
            continue;
        }
        refs += 1;
        bytes += stream.content.len() as u64;
        distinct.insert(Sha256::digest(&stream.content).to_vec());
        if let Ok(names) = stream.filters() {
            for name in names {
                filters.insert(String::from_utf8_lossy(name).into_owned());
            }
        }
    }
    (refs, distinct.len(), bytes, filters)
}

/// Whether every page's content stream is FlateDecode.
fn compressed(doc: &Document, pages: &BTreeMap<u32, ObjectId>) -> bool {
    pages.values().all(|page| {
        doc.get_page_contents(*page).iter().all(|id| {
            doc.get_object(*id)
                .and_then(Object::as_stream)
                .ok()
                .and_then(|stream| stream.filters().ok())
                .is_some_and(|filters| filters.iter().any(|name| *name == b"FlateDecode"))
        })
    })
}
