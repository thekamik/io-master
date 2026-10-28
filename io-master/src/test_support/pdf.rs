//! Help for the tests that run the tools of Poppler (`pdf/poppler.rs`): a
//! small but real PDF.

/// A small but real PDF with one page per text (Helvetica; a line
/// break in a text starts a new line), for the tests that run
/// `pdftotext`.
pub fn pdf(pages: &[&str]) -> Vec<u8> {
    let font = 3 + pages.len() * 2;
    let kids: Vec<String> = (0..pages.len()).map(|index| format!("{} 0 R", 3 + index * 2)).collect();

    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        format!("<< /Type /Pages /Kids [{}] /Count {} >>", kids.join(" "), pages.len()),
    ];

    for (index, text) in pages.iter().enumerate() {
        let lines: Vec<String> = text.split('\n').map(|line| format!("({line}) Tj 0 -14 Td")).collect();
        let content = format!("BT /F1 12 Tf 72 720 Td {} ET", lines.join(" "));

        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 {font} 0 R >> >> \
             /Contents {} 0 R >>",
            4 + index * 2
        ));
        objects.push(format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()));
    }

    objects.push("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned());

    let mut file = String::from("%PDF-1.4\n");
    let mut offsets = Vec::new();

    for (number, object) in objects.iter().enumerate() {
        offsets.push(file.len());
        file.push_str(&format!("{} 0 obj\n{object}\nendobj\n", number + 1));
    }

    let xref = file.len();
    file.push_str(&format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1));

    for offset in offsets {
        file.push_str(&format!("{offset:010} 00000 n \n"));
    }

    file.push_str(&format!(
        "trailer\n<< /Size {} /Root 1 0 R /Info << /Title (Test standard) >> >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    ));

    file.into_bytes()
}
