// @ts-check
import { classifySystemd, classifyDocker } from "./classify.js?v=2";
import { pluckPill } from "./pluck.js?v=1";

let LABEL_RULES = {};
let PILL_SPECS = {};

function applyRules(name, rules) {
    if (!rules) return name.replace(/\.(service|target)$/, "");
    let n = name.replace(/\.(service|target)$/, "");
    if (rules.strip_prefix && n.startsWith(rules.strip_prefix)) n = n.slice(rules.strip_prefix.length);
    if (rules.strip_suffix && n.endsWith(rules.strip_suffix)) n = n.slice(0, -rules.strip_suffix.length);
    if (rules.aliases && rules.aliases[n]) n = rules.aliases[n];
    return n;
}

const ATTENTION = { bad: 0, ghost: 1};

class StatusStrip extends HTMLElement {
    connectedCallback() {
        this._render({});
        this._tick();
        this._timer = setInterval(() => this._tick(), 30000);
    }

    disconnectedCallback() {
        clearInterval(this._timer);
    }

    async _tick() {
        try {
            const res = await fetch("/api/status");
            this._render(await res.json());
        } catch {
            // last render
        }
    }

    _render(status) {
        const zones = [];
        const attention = [];
        let up = 0, down = 0;

        for (const [name, entry] of Object.entries(status).sort()) {
            const rules = LABEL_RULES[name];
            const pills = [];
            let zoneDown = false;

            if (entry.error != null || entry.value == null) {
                zoneDown = true;
            } else if (entry.kind === "docker") {
                for (const c of entry.value) pills.push({ label: c.name, cls: classifyDocker(c), tip: c.status, raw: c.name });
            } else if (entry.kind === "systemd") {
                for (const u of entry.value) pills.push({ label: applyRules(u.name, rules), cls: classifySystemd(u), tip: u.name, raw: u.name });
            } else if (entry.kind === "pihole") {
                const q = entry.value.queries || {}, cl = entry.value.clients || {};
                pills.push({ label: `${Math.round(q.percent_blocked ?? 0)}% blocked`, cls: "ok", tip: `${q.blocked}/${q.total}`, raw: "" });
                pills.push({ label: `${cl.active} clients`, cls: "idle", tip: `${cl.total} known`, raw: "" });
            } else if (entry.kind === "unifi") {
                for (const d of entry.value.devices || []) {
                    const cls = d.state === "ONLINE" ? (d.firmware_updatable ? "ghost" : "ok") : "bad";
                    pills.push({ label: d.name, cls, tip: `${d.model}${d.firmware_updatable ? " · update available" : ""}`, raw: d.name });
                }
                pills.push({ label: `${entry.value.clients} clients`, cls: "idle", tip: "", raw: "" });
            } else {
                const specs = PILL_SPECS[name];
                if (specs && specs.length) {
                    for (const spec of specs) pills.push({...pluckPill(entry.value, spec), raw: ""});
                } else {
                    pills.push({label: "up", cls: "idle", tip: `fetched ${entry.fetched_at}`, raw: ""});
                }
            }

            // tally + collect attention items
            if (zoneDown) {
                down++;
                attention.push({ zone: name, label: "source down", cls: "bad", prio: 0 });
            } else {
                up++;
                for (const p of pills) {
                    if (p.cls in ATTENTION) {
                        attention.push({ zone: name, label: p.label, cls: p.cls, prio: ATTENTION[p.cls] });
                    }
                }
            }

            const okCount = pills.filter(p => p.cls === "ok" || p.cls === "idle").length;
            zones.push({ name, zoneDown, pills, count: `${okCount}/${pills.length}` });
        }

        attention.sort((a, b) => a.prio - b.prio);
        this._paint(zones, attention, { up, down, total: up + down });
    }

    _paint(zones, attention, totals) {
        const el = (tag, cls, txt) => {
            const e = document.createElement(tag);
            if (cls) e.className = cls;
            if (txt != null) e.textContent = txt;
            return e;
        };

        this.replaceChildren();

        // masthead totals
        const bar = el("div", "strip-totals");
        bar.append(
            el("span", "t-title", "sources:"),
            el("span", "t-up", `${totals.up} up`),
            el("span", "t-down", `${totals.down} down`),
            el("span", "t-total", `${totals.total} total`),
        );
        this.appendChild(bar);

        // attention row — only when something qualifies
        if (attention.length) {
            const row = el("div", "attention");
            row.appendChild(el("span", "attn-label", "NEEDS ATTENTION"));
            for (const a of attention) {
                const pill = el("span", "pill " + a.cls, a.label);
                pill.title = a.zone;
                const chip = el("span", "att-item");
                chip.append(pill, el("span", "att-zone", a.zone));
                row.appendChild(chip);
            }
            this.appendChild(row);
        }

        // zones, row-per-source with label column
        for (const z of zones) {
            const row = el("div", "zone" + (z.zoneDown ? " down" : ""));
            const label = el("span", "zlabel", z.name);
            const count = el("span", "zcount", z.zoneDown ? "down" : z.count);
            const head = el("div", "zhead");
            head.append(label, count);
            row.appendChild(head);

            const body = el("div", "zbody");
            for (const p of z.pills) {
                const pill = el("span", "pill " + p.cls, p.label);
                if (p.tip) pill.title = p.tip;
                body.appendChild(pill);
            }
            row.appendChild(body);
            this.appendChild(row);
        }
    }
}
customElements.define("status-strip", StatusStrip);

class SitrepPane extends HTMLElement {
    connectedCallback() {
        const title = this.getAttribute("title") || "";
        const url = this.getAttribute("url") || "";
        const kind = this.getAttribute("kind") || "site";
        this.setAttribute("kind", kind);

        this.innerHTML = `
            <div class="bar">
                <span>${title}</span>
                <button type="button" title="reload">&#8635;</button>
            </div>
            <div class="frame-wrap">
                <iframe src="${url}" loading="lazy" referrerpolicy="no-referrer"></iframe>
                <div class="veil">pane block or unreachable &mdash; check VPN, DNS &amp; targets frame headers</div>
            </div>
        `;

        const frame = this.querySelector("iframe");
        const wrap = this.querySelector(".frame-wrap");

        this.querySelector("button").addEventListener("click", () => {
            this.classList.remove("dead");
            this._arm();
            frame.src = url;
        });

        if (kind === "site") {
            const ro = new ResizeObserver(() => {
                const w = wrap.clientWidth;
                const render = parseFloat(getComputedStyle(this).getPropertyValue("--pane-render-width")) || 1280;
                this.style.setProperty("--scale", String(w / render));
            });
            ro.observe(wrap);
        }

        frame.addEventListener("load", () => {
            clearTimeout(this._deadline);
            this.classList.remove("dead");
        });

        this._arm();
    }

    _arm() {
        clearTimeout(this._deadline);
        this._deadline = setTimeout(() => this.classList.add("dead"), 8000);
    }
}

customElements.define("sitrep-pane", SitrepPane)

async function boot() {
    const cfg = await (await fetch("/api/config")).json();
    LABEL_RULES = cfg.label_rules || {};
    PILL_SPECS = cfg.pill_specs || {};

    for (const [key, value] of Object.entries(cfg.theme || {})) {
        document.documentElement.style.setProperty(`--${key}`, value);
    }

    const root = document.getElementById("panes");
    for (const p of cfg.layout?.panes || []) {
        const el = document.createElement("sitrep-pane");
        el.setAttribute("title", p.title);
        el.setAttribute("url", p.url);
        el.setAttribute("kind", p.kind);
        root.appendChild(el);
    }
}

boot();