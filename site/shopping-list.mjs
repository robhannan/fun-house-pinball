// Reads the parts tables in docs/shopping-list.md for the parts tracker.
// Only tables with "Item" and "Qty" columns count; a Qty of "—" means the
// part comes in another kit, so the row is skipped.

const plain = (text) =>
  text
    .replace(/\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replace(/[*`]/g, "")
    .replace(/🛡️\s*/g, "")
    .trim();

const slug = (text) =>
  text
    .toLowerCase()
    .normalize("NFKD")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");

const cells = (line) =>
  line
    .trim()
    .replace(/^\||\|$/g, "")
    .split("|")
    .map((cell) => cell.trim());

const links = (text) =>
  [...text.matchAll(/\[([^\]]+)\]\(([^)]+)\)/g)].map(([, label, url]) => ({ label, url }));

// "2", "1 pack each", "3–5", "about 25 parts" or "As needed". A range counts
// as its upper end; "As needed" has no target.
export function parseQty(text) {
  const qty = plain(text);
  if (/^as needed$/i.test(qty)) return { need: null, qty };
  const match = qty.match(/^(?:about\s+)?(\d+)(?:\s*[–-]\s*(\d+))?(?:\s+.*)?$/i);
  if (!match) return undefined;
  return { need: Number(match[2] ?? match[1]), qty };
}

const firstSentence = (text) => {
  const match = text.match(/^.*?[.!?](?=\s|$)/);
  return match ? match[0] : text;
};

export function parseShoppingList(markdown) {
  const sections = [];
  const errors = [];
  const ids = new Set();
  let section = null;
  let group = null;
  let columns = null;

  markdown.split("\n").forEach((line, index) => {
    const lineNo = index + 1;
    const h2 = line.match(/^## (.+)/);
    const h3 = line.match(/^### (.+)/);
    if (h2) {
      section = { key: slug(h2[1].split(" ")[0]), title: plain(h2[1]), groups: [] };
      sections.push(section);
      group = null;
      columns = null;
      return;
    }
    if (h3 && section) {
      group = { title: plain(h3[1]), items: [] };
      section.groups.push(group);
      columns = null;
      return;
    }
    if (!line.startsWith("|")) {
      columns = null;
      return;
    }
    const row = cells(line);
    if (row.every((cell) => /^:?-+:?$/.test(cell))) return;
    if (!columns) {
      columns = Object.fromEntries(row.map((name, i) => [name.toLowerCase(), i]));
      if (!("item" in columns && "qty" in columns)) columns = { skip: true };
      return;
    }
    if (columns.skip || !section) return;

    const itemText = row[columns.item] ?? "";
    const qtyText = row[columns.qty] ?? "";
    if (plain(qtyText) === "—") return;
    const qty = parseQty(qtyText);
    if (!qty) {
      errors.push(`line ${lineNo}: can't read the quantity "${qtyText}" for "${plain(itemText)}"`);
      return;
    }
    const optional = /^\*Optional:\*\s*/.test(itemText);
    const name = plain(itemText.replace(/^\*Optional:\*\s*/, ""));
    let id = slug(name);
    if (ids.has(id)) id = `${id}-${section.key}`;
    if (ids.has(id)) {
      errors.push(`line ${lineNo}: "${name}" appears twice in section ${section.key}`);
      return;
    }
    ids.add(id);

    if (!group) {
      group = { title: null, items: [] };
      section.groups.push(group);
    }
    group.items.push({
      id,
      name,
      need: qty.need,
      qty: qty.qty,
      optional,
      distributor: itemText.includes("🛡️"),
      note: plain(firstSentence(row[columns.notes] ?? "")),
      links: "link" in columns ? links(row[columns.link] ?? "") : [],
      where: "link" in columns && !links(row[columns.link] ?? "").length ? plain(row[columns.link] ?? "") : "",
    });
  });

  for (const s of sections) s.groups = s.groups.filter((g) => g.items.length);
  return { sections: sections.filter((s) => s.groups.length), errors };
}
