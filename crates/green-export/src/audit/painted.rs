//! How much of a page one picture paints: the content stream walked with its
//! transformation matrix. Typst writes pictures as image XObjects placed by
//! `cm … Do`, forms with their own matrix and resources, so both are followed.

use std::collections::BTreeMap;

use lopdf::content::Content;
use lopdf::{Dictionary, Document, Object, ObjectId};

/// The page whose widest picture covers most of it, and that share.
pub(super) fn widest(doc: &Document, pages: &BTreeMap<u32, ObjectId>) -> (Option<u32>, f64) {
    let mut widest = (None, 0.0);
    for (number, page) in pages {
        let Some(boundaries) = media_box(doc, *page, 0) else {
            continue;
        };
        let area = (boundaries[2] - boundaries[0]) * (boundaries[3] - boundaries[1]);
        if area <= 0.0 {
            continue;
        }
        let Some(resources) = resources(doc, *page, 0) else {
            continue;
        };
        let content = doc.get_page_content(*page);
        let share = paint(doc, &content, resources, 1.0, area as f64, 0);
        if share > widest.1 {
            widest = (Some(*number), share);
        }
    }
    widest
}

fn media_box(doc: &Document, page: ObjectId, depth: usize) -> Option<[f32; 4]> {
    if depth > 32 {
        return None;
    }
    let dict = doc.get_dictionary(page).ok()?;
    match dict.get(b"MediaBox").ok() {
        Some(object) => numbers(doc, object),
        None => media_box(
            doc,
            dict.get(b"Parent").ok()?.as_reference().ok()?,
            depth + 1,
        ),
    }
}

fn resources(doc: &Document, page: ObjectId, depth: usize) -> Option<&Dictionary> {
    if depth > 32 {
        return None;
    }
    let dict = doc.get_dictionary(page).ok()?;
    match dict.get(b"Resources").ok()? {
        Object::Reference(id) => doc.get_dictionary(*id).ok(),
        Object::Dictionary(resources) => Some(resources),
        _ => None,
    }
    .or_else(|| {
        let parent = dict.get(b"Parent").ok()?.as_reference().ok()?;
        resources(doc, parent, depth + 1)
    })
}

/// The largest picture area in `content`, as a share of `area`. `scale` is the
/// determinant of the transformation matrix in force; the unit square a `Do`
/// paints covers that many square points.
fn paint(
    doc: &Document,
    content: &[u8],
    resources: &Dictionary,
    scale: f64,
    area: f64,
    depth: usize,
) -> f64 {
    if depth > 8 {
        return 0.0;
    }
    let Ok(content) = Content::decode(content) else {
        return 0.0;
    };
    let xobjects = match resources.get(b"XObject").ok() {
        Some(Object::Reference(id)) => doc.get_dictionary(*id).ok(),
        Some(Object::Dictionary(dict)) => Some(dict),
        _ => None,
    };
    let (mut current, mut best, mut saved) = (scale, 0.0_f64, Vec::new());
    for operation in &content.operations {
        match operation.operator.as_str() {
            "q" => saved.push(current),
            "Q" => {
                if let Some(previous) = saved.pop() {
                    current = previous;
                }
            }
            "cm" => {
                if let Some(matrix) = matrix(operation.operands.as_slice()) {
                    current *= (matrix[0] * matrix[3] - matrix[1] * matrix[2]).abs();
                }
            }
            "Do" => {
                let Some(xobjects) = xobjects else { continue };
                let Some(name) = operation
                    .operands
                    .first()
                    .and_then(|name| name.as_name().ok())
                else {
                    continue;
                };
                let Some(target) = xobjects.get(name).ok().and_then(|object| match object {
                    Object::Reference(id) => doc.get_object(*id).ok(),
                    other => Some(other),
                }) else {
                    continue;
                };
                let Object::Stream(stream) = target else {
                    continue;
                };
                let subtype = stream.dict.get(b"Subtype").and_then(Object::as_name).ok();
                match subtype {
                    Some(name) if name == b"Image" => {
                        best = best.max((current / area).min(1.0));
                    }
                    Some(name) if name == b"Form" => {
                        let inner = stream
                            .dict
                            .get(b"Matrix")
                            .ok()
                            .and_then(|matrix| numbers(doc, matrix))
                            .map_or(current, |matrix| {
                                current
                                    * (matrix[0] * matrix[3] - matrix[1] * matrix[2]).abs() as f64
                            });
                        let own = match stream.dict.get(b"Resources").ok() {
                            Some(Object::Reference(id)) => doc.get_dictionary(*id).ok(),
                            Some(Object::Dictionary(dict)) => Some(dict),
                            _ => Some(resources),
                        };
                        if let Some(own) = own {
                            let Ok(bytes) = stream.decompressed_content() else {
                                continue;
                            };
                            best = best.max(paint(doc, &bytes, own, inner, area, depth + 1));
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    best
}

fn matrix(operands: &[Object]) -> Option<[f64; 6]> {
    if operands.len() < 6 {
        return None;
    }
    let mut matrix = [0.0; 6];
    for (slot, operand) in matrix.iter_mut().zip(operands) {
        *slot = match operand {
            Object::Real(value) => f64::from(*value),
            Object::Integer(value) => *value as f64,
            _ => return None,
        };
    }
    Some(matrix)
}

fn numbers(doc: &Document, object: &Object) -> Option<[f32; 4]> {
    let array = match object {
        Object::Reference(id) => doc.get_object(*id).ok()?.as_array().ok()?,
        Object::Array(array) => array,
        _ => return None,
    };
    if array.len() < 4 {
        return None;
    }
    let mut box_ = [0.0; 4];
    for (slot, item) in box_.iter_mut().zip(array) {
        *slot = match item {
            Object::Real(value) => *value,
            Object::Integer(value) => *value as f32,
            _ => return None,
        };
    }
    Some(box_)
}
