use hayagriva::{BufWriteFormat, ElemChildren};

pub(crate) fn elem_children_to_string(
    children: &ElemChildren,
    format: BufWriteFormat,
) -> Result<String, String> {
    let mut output = String::new();
    children
        .write_buf(&mut output, format)
        .map_err(|err| err.to_string())?;
    Ok(output)
}
