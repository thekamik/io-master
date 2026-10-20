"""Turns the text of CEM:2022 R1 into an OSCAL 1.2.3 catalog.

    pdftohtml -xml -i -q CEM2022R1.pdf cem
    python3 -I cemxml2text.py cem.xml cem-para.txt
    python3 -I cem2oscal.py cem-para.txt CEM2022R1.pdf out.json

Structure: class -> family -> sub-activity -> action -> work unit (control)."""
import hashlib, json, re, sys, uuid, datetime, collections

text_path, pdf_path, out_path = sys.argv[1:4]
NS = "https://github.com/thekamik/io-master/ns/cem"
NAMESPACE = uuid.UUID("6f1d1f4e-6a53-4c1e-9d0e-2a3c6c0e5c11")  # stable UUIDs: same input, same file


def uid(*parts):
    return str(uuid.uuid5(NAMESPACE, "/".join(parts)))


raw = open(text_path, encoding="utf-8").read().split("\n")

# --- 1. The lines of the body (cemxml2text.py has left out the heads and feet of the pages,
# and a blank line ends a paragraph). ---
lines = [line.rstrip() for line in raw]

# A heading of a family broken over two lines ("15.10 Integration ...\n procedures (ALC_COMP)") is joined.
joined = []
for line in lines:
    if (joined and re.match(r"^1[0-8]\.\d+ [A-Z]", joined[-1].strip()) and not joined[-1].strip().endswith(")")
            and re.search(r"\([A-Z]{3}_[A-Z]{3,4}\)$", line.strip())):
        joined[-1] = joined[-1].strip() + " " + line.strip()
    else:
        joined.append(line)
lines = joined

H_FAMILY = re.compile(r"^(1[0-8])\.(\d+) (.+) \(([A-Z]{3}_[A-Z]{3,4})\)$")
H_SUB = re.compile(r"^(1[0-8])\.(\d+)\.(\d+) +Evaluation of sub-activity \(([A-Z]{3}_[A-Z]{3,4}\.\d+)\)$")
H_SUBPART = re.compile(r"^(1[0-8])\.\d+\.\d+\.\d+ +(Objectives|Input|Application notes)$")
H_ACTION = re.compile(r"^(1[0-8])\.\d+\.\d+\.\d+ +Action ([A-Z]{3}_[A-Z]{3,4}\.\d+\.\d+E)$")
H_ACTION_SUB = re.compile(r"^(1[0-8])\.\d+\.\d+\.\d+\.\d+ +(General|Application notes)$")
H_WORK = re.compile(r"^(1[0-8])\.\d+\.\d+\.\d+\.\d+ +Work unit ([A-Z]{3}_[A-Z]{3,4}\.\d+-\d+)$")
H_ANY = re.compile(r"^(\d{1,2}(\.\d+)+|[A-Z]\.\d+(\.\d+)*) +[A-Z]")
CC_ELEMENT = re.compile(r"^CC Part 3 ([A-Z]{3}_[A-Z]{3,4}\.\d+\.\d+[CDE]): ?(.*)$")
# The titles of the classes, from the table of contents.
CLASS_TITLES = {}
for line in raw:
    m = re.match(r"^\s*Class ([A-Z]{3}): ([A-Za-z ,\-]+?)\s*\.{3,}", line)
    if m:
        CLASS_TITLES.setdefault(m.group(1), m.group(2).strip())


def paragraphs(block):
    """The lines of a block joined into paragraphs: a paragraph ends on a blank line (a gap
    on the page, cemxml2text.py) or before an item of a list ("a)")."""
    out, cur = [], []
    for line in block:
        s = re.sub(r"\s+", " ", line.strip())
        if not s:
            if cur:
                out.append(" ".join(cur)); cur = []
            continue
        if re.match(r"^([a-z]|[ivx]+|\d+)\) ", s) or s.startswith("• ") or s.startswith("— "):
            if cur:
                out.append(" ".join(cur)); cur = []
        cur.append(s)
    if cur:
        out.append(" ".join(cur))
    return [re.sub(r"(\w)- (\w)", r"\1-\2", p) for p in out if p]


def prose(paras):
    out = ""
    for p in paras:
        if out:
            out += "\n" if re.match(r"^([a-z]|[ivx]+|\d+)\) ", p) else "\n\n"
        out += p
    return out


def statement_and_guidance(block):
    """The work unit itself is its first paragraph, with the items of a list that
    follows a colon; the rest is guidance."""
    paras = paragraphs(block)
    if not paras:
        return "", []
    stmt = [paras[0]]
    i = 1
    if paras[0].endswith(":"):
        while i < len(paras) and re.match(r"^([a-z]|[ivx]+|\d+)\) ", paras[i]):
            stmt.append(paras[i]); i += 1
    return prose(stmt), paras[i:]


# The work units in which the laboratory carries out tests on the TOE (method TEST): devising the tests,
# writing their documentation, running them, recording their actual results, and carrying out the
# procedures of the guidance on the TOE. Searching, analysing, checking and reporting are EXAMINE.
TEST_RULES = [
    r"shall (conduct|perform) (penetration )?testing",
    r"shall perform testing",
    r"shall devise (a test subset|penetration tests)",
    r"shall produce (penetration )?test documentation",
    r"shall record the actual results of the penetration tests",
    r"shall record the following information about the tests",
    r"shall specify, conduct and document penetration testing",
    r"shall perform all user procedures",
    r"^The tests are to be selected and executed",
]


def method_of(stmt):
    first = stmt.split("\n")[0]
    if any(re.search(rule, first) for rule in TEST_RULES):
        return "TEST"
    return "EXAMINE"


def title_of(stmt, wid):
    """The first sentence of the work unit without "The evaluator shall", shortened on a word:
    "Examine the PP reference to determine that it uniquely identifies the PP"."""
    first = re.split(r"(?<=[.:])\s", stmt.split("\n")[0])[0].rstrip(".:")
    m = re.search(r"\bthe evaluator shall (.+)", first, re.I)
    if not m:
        return f"Work unit {wid}"
    words, title = m.group(1).split(), ""
    for word in words:
        if len(title) + len(word) > 110:
            title += " ..."
            break
        title += (" " if title else "") + word
    return title[0].upper() + title[1:]


METHOD_PROSE = {
    "TEST": "TEST: the laboratory carries out tests on the TOE in this work unit.",
    "EXAMINE": "EXAMINE: the evaluator examines, checks or analyses the evidence and reports.",
}


def verb_of(stmt):
    m = re.search(r"\bshall ([a-z]+)", stmt)
    return m.group(1) if m else ""


# --- 2. Walk the body. ---
classes = collections.OrderedDict()
fam = sub = act = None
cur_kind, cur_block, pending_cc = None, [], []
started = False


def flush():
    global cur_kind, cur_block
    kind, block = cur_kind, cur_block
    cur_kind, cur_block = None, []
    if kind is None:
        return
    if kind in ("Objectives", "Input", "Application notes") and sub is not None:
        sub["parts"].append((kind, prose(paragraphs(block))))
    elif kind == "action-general" and act is not None:
        act["general"] = block
    elif kind == "action-notes" and act is not None:
        act["notes"].append(prose(paragraphs(block)))
    elif kind == "work" and act is not None:
        wu = act["work"][-1]
        stmt, rest = statement_and_guidance(block)
        # A "CC Part 3 ..." paragraph inside the guidance belongs to the next work unit.
        guidance, carry = [], []
        for p in rest:
            m = CC_ELEMENT.match(p)
            if m or carry:
                carry.append(p)
            else:
                guidance.append(p)
        wu["statement"], wu["guidance"] = stmt, prose(guidance)
        pending_cc.extend(carry)


for line in lines:
    s = line.strip()
    m_family = H_FAMILY.match(s)
    if m_family and int(m_family.group(1)) >= 10:
        started = True
        flush()
        code = m_family.group(4)
        cl = code[:3]
        if cl not in classes:
            classes[cl] = {"id": cl, "title": CLASS_TITLES.get(cl, cl), "families": collections.OrderedDict()}
        fam = {"id": code, "title": m_family.group(3), "subs": []}
        classes[cl]["families"][code] = fam
        sub = act = None
        continue
    if not started:
        continue
    if re.match(r"^(1[0-8])\.\d+ [A-Z]", s) and not m_family:
        # A section of a class that is not a family (General, Application notes, ACE_* ...): end the family.
        flush(); fam = sub = act = None
        continue
    m = H_SUB.match(s)
    if m and fam is not None:
        flush()
        sub = {"id": m.group(4), "title": f"Evaluation of sub-activity ({m.group(4)})", "parts": [], "actions": []}
        fam["subs"].append(sub); act = None
        continue
    m = H_SUBPART.match(s)
    if m and sub is not None:
        flush(); cur_kind = m.group(2); act = None
        continue
    m = H_ACTION.match(s)
    if m and sub is not None:
        flush()
        act = {"id": m.group(2), "general": [], "notes": [], "work": []}
        sub["actions"].append(act); pending_cc.clear()
        continue
    m = H_ACTION_SUB.match(s)
    if m and act is not None:
        flush(); cur_kind = "action-general" if m.group(2) == "General" else "action-notes"
        continue
    m = H_WORK.match(s)
    if m and act is not None:
        flush()
        if not pending_cc and act["general"]:
            # The first work unit of an action: its elements of CC Part 3 are in "General" (from the
            # first "CC Part 3 ..." paragraph on).
            general = paragraphs(act["general"])
            first = next((i for i, p in enumerate(general) if CC_ELEMENT.match(p)), len(general))
            pending_cc.extend(general[first:])
            act["general"] = []
        label = m.group(2)
        number = label.rsplit("-", 1)[1]
        # The CEM numbers a work unit of ACE_OBJ.1 "APE_OBJ.1-3" (a slip of its text): the id follows the
        # sub-activity, the label of the CEM is kept.
        wid = label if label.startswith(sub["id"] + "-") else f"{sub['id']}-{number}"
        act["work"].append({"id": wid, "label": label, "cc": list(pending_cc)})
        pending_cc.clear()
        cur_kind = "work"
        continue
    if H_ANY.match(s) and re.match(r"^(1[0-8])\.", s) and act is not None and cur_kind == "work":
        flush()
        continue
    if re.match(r"^(19|2\d) |^Annex ", s) or re.match(r"^Bibliography$", s):
        flush(); break
    if cur_kind:
        cur_block.append(line)
flush()


# --- 3. OSCAL. ---
def prop(name, value, ns=True):
    p = {"name": name, "value": value}
    if ns:
        p["ns"] = NS
    return p


def cc_parts(cc_paras):
    """The elements of CC Part 3 a work unit checks: ids and texts."""
    ids, texts = [], []
    for p in cc_paras:
        m = CC_ELEMENT.match(p)
        if m:
            ids.append(m.group(1)); texts.append(f"{m.group(1)}: {m.group(2)}".strip())
        elif texts:
            texts[-1] += ("\n" if re.match(r"^([a-z]|[ivx]+|\d+)\) ", p) else " ") + p
    return ids, texts


counts = collections.Counter()
groups = []
for cl in classes.values():
    fam_groups = []
    for fam in cl["families"].values():
        sub_groups = []
        for sub in fam["subs"]:
            act_groups = []
            for act in sub["actions"]:
                controls = []
                for wu in act["work"]:
                    if not wu.get("statement"):
                        continue
                    method = method_of(wu["statement"])
                    counts[method] += 1
                    ids, texts = cc_parts(wu["cc"])
                    props = [prop("label", wu["label"], ns=False), prop("action-element", act["id"])]
                    if wu["label"] != wu["id"]:
                        props.append(prop("cem-numbering", f"numbered {wu['label']} in the CEM, a slip of its text"))
                    props += [prop("cc-element", i) for i in ids]
                    if verb_of(wu["statement"]):  # ADV_SPM.1-11 and ACO_CTT.2-9 have no "shall ..."
                        props.append(prop("verb", verb_of(wu["statement"])))
                    parts = [{"id": f"{wu['id']}_smt", "name": "statement", "prose": wu["statement"]}]
                    for t in texts:
                        parts.append({"name": "cc-element", "ns": NS, "prose": t})
                    parts.append({
                        "id": f"{wu['id']}_asm", "name": "assessment-method",
                        "props": [prop("method", method, ns=False)],
                        "prose": METHOD_PROSE[method],
                    })
                    if wu["guidance"]:
                        parts.append({"id": f"{wu['id']}_gdn", "name": "guidance", "prose": wu["guidance"]})
                    controls.append({"id": wu["id"], "class": "work-unit", "title": title_of(wu["statement"], wu["id"]),
                                     "props": props, "parts": parts})
                if not controls:
                    continue
                g = {"id": act["id"], "class": "action", "title": f"Action {act['id']}", "controls": controls}
                notes = [n for n in act["notes"] if n]
                if notes:
                    g["parts"] = [{"name": "application-notes", "ns": NS, "prose": "\n\n".join(notes)}]
                act_groups.append(g)
            if not act_groups:
                continue
            g = {"id": sub["id"], "class": "sub-activity", "title": sub["title"]}
            parts = []
            for kind, text in sub["parts"]:
                if text:
                    parts.append({"name": {"Objectives": "objective", "Input": "input",
                                           "Application notes": "application-notes"}[kind], "ns": NS, "prose": text})
            if parts:
                g["parts"] = parts
            g["groups"] = act_groups
            sub_groups.append(g)
        if sub_groups:
            fam_groups.append({"id": fam["id"], "class": "family", "title": fam["title"], "groups": sub_groups})
    if fam_groups:
        groups.append({"id": cl["id"], "class": "class", "title": cl["title"], "groups": fam_groups})

pdf = open(pdf_path, "rb").read()
sha = hashlib.sha256(pdf).hexdigest()
now = datetime.datetime.now(datetime.timezone.utc).replace(microsecond=0).isoformat()
total = sum(counts.values())

catalog = {"catalog": {
    "uuid": uid("catalog"),
    "metadata": {
        "title": "Common Methodology for Information Technology Security Evaluation (CEM:2022 Revision 1): work units",
        "published": "2022-11-01T00:00:00+00:00",
        "last-modified": now,
        "version": "CEM:2022 Revision 1 (CCMB-2022-11-006)",
        "oscal-version": "1.2.3",
        "props": [prop("work-units", str(total)), prop("work-units-test", str(counts["TEST"]))],
        "links": [{"href": "https://www.commoncriteriaportal.org/files/ccfiles/CEM2022R1.pdf", "rel": "alternate",
                   "media-type": "application/pdf", "text": "CEM:2022 Revision 1 on the Common Criteria portal"}],
        "remarks": (
            "The work units of the CEM:2022 R1 (the methodology of Common Criteria, ISO/IEC 18045), one control per "
            "work unit, in the structure of the CEM: class (group) -> family (group) -> evaluation sub-activity (group) -> "
            "evaluator action (group) -> work unit (control). The statement of a control is the text of the work unit; "
            "its assessment method is TEST where the laboratory carries out tests on the TOE (devises them, writes their "
            "documentation, runs them, records their results, carries out the procedures of the guidance), else "
            "EXAMINE; the elements of CC Part 3 a work unit checks are named in its props and quoted in its parts; the explanations of the CEM are its guidance.\n\n"
            "Made from the text of the PDF by a script; to be checked against the PDF before use in an evaluation.\n\n"
            "Legal notice of the CEM: the governmental organizations that hold its copyright \"have always been "
            "supportive of the text being reused by any users of the document, including modifications and reuse of "
            "part of the document\"."),
    },
    "groups": groups,
    "back-matter": {"resources": [{
        "uuid": uid("resource", sha),
        "title": "Common Methodology for Information Technology Security Evaluation (CEM:2022 Revision 1)",
        "rlinks": [{"href": "CEM2022R1.pdf", "media-type": "application/pdf",
                    "hashes": [{"algorithm": "SHA-256", "value": sha}]}],
    }]},
}}

json.dump(catalog, open(out_path, "w", encoding="utf-8"), ensure_ascii=False, indent=2)
print("work units:", total, dict(counts))
for g in groups:
    print(g["id"], sum(len(a["controls"]) for f in g["groups"] for s in f["groups"] for a in s["groups"]))
