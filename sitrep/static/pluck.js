// @ts-check

// Resolve a dotted path into a JSON value. Supports object keys and array indices, including negative indices (from the end).
// Returns undefined on any miss - never throws. 
export function pluck(value, path) {
    const parts = path.split(".");
    let cur = value;
    for (const part of parts) {
        if (cur == null) return undefined;
        if (Array.isArray(cur)) {
            let i = Number(part);
            if (!Number.isInteger(i)) return undefined;
            if (i < 0) i += cur.length;
            if (i < 0 || i >= cur.length) return undefined;
            cur = cur[i];
        } else if (typeof cur === "object") {
            if (!(part in cur)) return undefined;
            cur = cur[part];
        } else {
            return undefined;
        }
    }
    return cur;
}

// Build one pill from a spec + the source's raw value.
// spec: { path, label, state, map }
export function pluckPill(value, spec) {
    const raw = pluck(value, spec.path);
    const shown = raw == null ? "?" : String(raw);
    const label = (spec.label || "{}").replace("{}", shown);

    let cls;
    if (spec.map) {
        cls = spec.map[shown] ?? "ghost";       // unmapped -> ghost (Honest Default)
    } else {
        cls = spec.state || "idle";
    }
    if (raw == null) cls = "ghost";      // missing value is unknown

    return { label, cls, tip: spec.path };
}