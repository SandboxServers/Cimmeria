"""Markdown cells and sections for the campaign report: a missing value shows its gap, never a zero."""


def cell(value, gap=None, fmt=str):
    if value is None:
        return "-" if gap is None else "n/a (" + gap + ")"
    return fmt(value)


def gap_of(section, field):
    return section.get(field + "_gap")


def section_md(title, body):
    return f"### {title}\n\n{body}\n"
