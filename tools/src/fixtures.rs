//! The fixtures as files: the four generated documents, the images, the manifest.
//!
//! Everything comes from `documents::SEED`, so two runs write the same bytes on
//! any platform; `fixtures --check` holds the committed copies to that.

use crate::documents;
use crate::png::Canvas;
use crate::sha256;

pub struct Artifact {
    pub path: &'static str,
    pub note: String,
    pub bytes: Vec<u8>,
}

impl Artifact {
    pub fn text(&self) -> Option<&str> {
        std::str::from_utf8(&self.bytes).ok()
    }

    pub fn chars(&self) -> usize {
        self.text().map_or(0, |text| text.chars().count())
    }

    pub fn lines(&self) -> usize {
        self.text().map_or(0, |text| text.lines().count())
    }

    /// Blank-line-separated blocks that are prose, not headings, tables or code.
    pub fn paragraphs(&self) -> usize {
        self.text().map_or(0, count_paragraphs)
    }
}

pub fn all() -> Vec<Artifact> {
    let paragraphs = documents::fifty_thousand_paragraphs();
    let one_line = documents::fifty_thousand_one_line();
    let large = documents::five_megabytes();
    let homework = documents::homework();
    vec![
        Artifact {
            path: "fixtures/perf/50k-paragraphs.md",
            note: format!(
                "5 万字、{} 段，正常 Markdown",
                count_paragraphs(&paragraphs)
            ),
            bytes: paragraphs.into_bytes(),
        },
        Artifact {
            path: "fixtures/perf/50k-single-paragraph.md",
            note: String::from("5 万字、单段一行、无换行（对抗用例）"),
            bytes: one_line.into_bytes(),
        },
        Artifact {
            path: "fixtures/perf/5mb-large.md",
            note: format!(
                "5 MB；正文 {} 字，其余为长表与日志填充",
                count_prose_chars(&large)
            ),
            bytes: large.into_bytes(),
        },
        Artifact {
            path: "fixtures/perf/homework-28-formulas.md",
            note: format!(
                "作业笔记，{} 条公式（docs/design.md §7）",
                documents::formulas(&homework).len()
            ),
            bytes: homework.into_bytes(),
        },
        Artifact {
            path: "fixtures/real/images/architecture.png",
            note: String::from("真实文档示例用的示意图（自造，400×225）"),
            bytes: architecture(),
        },
        Artifact {
            path: "fixtures/real/images/timeline.png",
            note: String::from("真实文档示例用的时间线图（自造，400×120）"),
            bytes: timeline(),
        },
    ]
}

/// Characters outside fenced code blocks and table rows.
fn count_prose_chars(text: &str) -> usize {
    let mut fenced = false;
    let mut chars = 0;
    for line in text.lines() {
        if line.starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced || line.starts_with('|') {
            continue;
        }
        chars += line.chars().count() + 1;
    }
    chars
}

fn count_paragraphs(text: &str) -> usize {
    text.split("\n\n")
        .filter(|block| {
            block
                .trim_start()
                .chars()
                .next()
                .is_some_and(|first| !matches!(first, '#' | '|' | '`' | '-' | '>'))
        })
        .count()
}

pub fn manifest(artifacts: &[Artifact]) -> String {
    let mut out = String::from(
        "# 生成夹具清单\n\n\
         由 `cargo run --manifest-path tools/Cargo.toml --bin fixtures` 写出，种子 `0x677265656E6F7465`；\
         不要手改，改生成器再重跑。`--check` 逐字节比对已提交的副本。\n\n\
         | 文件 | 说明 | 字节 | 字符 | 行 | 段 | SHA-256 |\n\
         | --- | --- | ---: | ---: | ---: | ---: | --- |\n",
    );
    for artifact in artifacts {
        let chars = if artifact.text().is_some() {
            artifact.chars().to_string()
        } else {
            String::from("—")
        };
        let lines = if artifact.text().is_some() {
            artifact.lines().to_string()
        } else {
            String::from("—")
        };
        let paragraphs = if artifact.text().is_some() {
            artifact.paragraphs().to_string()
        } else {
            String::from("—")
        };
        out.push_str(&format!(
            "| `{}` | {} | {} | {chars} | {lines} | {paragraphs} | `{}` |\n",
            artifact.path,
            artifact.note,
            artifact.bytes.len(),
            sha256::hex(&sha256::digest(&artifact.bytes)),
        ));
    }
    out
}
fn architecture() -> Vec<u8> {
    let mut canvas = Canvas::rgb(400, 225);
    let paper = [0xf7, 0xf5, 0xf0];
    let ink = [0x2b, 0x3a, 0x2f];
    let accent = [0x3f, 0x7a, 0x4d];
    canvas.rect(0, 0, 400, 225, paper);
    for (left, top, right, bottom) in [
        (24, 24, 150, 74),
        (24, 100, 150, 150),
        (238, 24, 376, 88),
        (238, 118, 376, 188),
    ] {
        canvas.rect(left, top, right, bottom, ink);
        canvas.rect(left + 3, top + 3, right - 3, bottom - 3, paper);
    }
    for y in 36..68 {
        canvas.rect(150, y, 238, y + 2, accent);
    }
    canvas.rect(74, 150, 77, 188, accent);
    canvas.rect(74, 188, 306, 191, accent);
    canvas.rect(303, 124, 306, 191, accent);
    canvas.png()
}

fn timeline() -> Vec<u8> {
    let mut canvas = Canvas::rgb(400, 120);
    let paper = [0xff, 0xff, 0xff];
    let ink = [0x22, 0x22, 0x22];
    canvas.rect(0, 0, 400, 120, paper);
    canvas.rect(24, 58, 376, 61, ink);
    for step in 0..5 {
        let x = 40 + step * 80;
        canvas.rect(x, 40, x + 14, 80, ink);
        canvas.rect(x + 3, 43, x + 11, 77, paper);
    }
    canvas.png()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fixtures_hit_their_sizes() {
        let artifacts = all();
        let paragraphs = artifacts[0].text().expect("text");
        assert_eq!(paragraphs.chars().count(), 50_000);
        assert!(
            (340..=360).contains(&artifacts[0].paragraphs()),
            "{}",
            artifacts[0].paragraphs()
        );

        let one_line = artifacts[1].text().expect("text");
        assert_eq!(one_line.chars().count(), 50_000);
        assert!(
            !one_line.contains('\n'),
            "the adversarial fixture is one line"
        );
        assert_eq!(artifacts[1].lines(), 1);

        assert!(
            (4_900_000..=5_100_000).contains(&artifacts[2].bytes.len()),
            "{}",
            artifacts[2].bytes.len()
        );

        let homework = artifacts[3].text().expect("text");
        assert_eq!(documents::FORMULAS.len(), 28);
        assert_eq!(documents::formulas(homework).len(), 28);
        assert!(homework.contains("==高亮=="));
    }

    #[test]
    fn a_stray_dollar_shows_up_in_the_count() {
        assert_eq!(documents::formulas("$a$ and $$b$$").len(), 2);
        assert_eq!(
            documents::formulas("价格 $5 与 $6"),
            Vec::<&str>::from(["5 与 "])
        );
    }

    #[test]
    fn two_runs_are_byte_identical() {
        let first = all();
        let second = all();
        for (left, right) in first.iter().zip(&second) {
            assert_eq!(left.bytes, right.bytes, "{} differs", left.path);
        }
    }
}
