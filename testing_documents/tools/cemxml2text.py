"""The text of the CEM from `pdftohtml -xml -i`: one line per line of the page, a blank line where a
paragraph ends (a vertical gap larger than the spacing of lines), without the head and the foot of the
pages and without footnotes (small fonts).

    python3 -I cemxml2text.py cem.xml cem-para.txt"""
import html, re, sys
import xml.etree.ElementTree as ET

src, out = sys.argv[1:3]
root = ET.parse(src).getroot()
sizes = {}
for spec in root.iter("fontspec"):
    sizes[spec.get("id")] = int(spec.get("size"))

result = []
for page in root.iter("page"):
    height = int(page.get("height"))
    rows = {}
    for t in page.iter("text"):
        top, left, width = int(t.get("top")), int(t.get("left")), int(t.get("width"))
        size = sizes.get(t.get("font"), 17)
        if top < 80 or top > height - 80:  # the head and the foot of the page
            continue
        if size < 16:  # footnotes and their marks
            continue
        text = "".join(t.itertext())
        key = next((k for k in rows if abs(k - top) <= 3), top)
        rows.setdefault(key, []).append((left, width, text))
    lines = []
    for top in sorted(rows):
        parts = sorted(rows[top])
        text = ""
        for left, width, piece in parts:
            text += piece if not text else (" " if not text.endswith(" ") and not piece.startswith(" ") else "") + piece
        text = re.sub(r"\s+", " ", text).strip()
        if text:
            right = max(l + w for l, w, _ in parts)
            lines.append((top, right, text))
    if not lines:
        continue
    # A paragraph that ends at the foot of the previous page: its last line is short and ends a sentence.
    if result and result[-1] != "":
        prev = result[-1]
        if prev_right < 740 and prev.endswith((".", ":", ";")):
            result.append("")
    prev_top = None
    for top, right, text in lines:
        if prev_top is not None and top - prev_top > 26:
            result.append("")
        result.append(text)
        prev_top, prev_right = top, right

open(out, "w", encoding="utf-8").write("\n".join(result))
