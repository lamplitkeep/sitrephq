// @ts-check
import { classifySystemd, classifyDocker } from "./classify.js?v=2";
import { pluckPill } from "./pluck.js?v=1";

let LABEL_RULES = {};
let PILL_SPECS = {};
let SOURCE_ORDER = [];

const ATTENTION = { bad: 0, ghost: 1};

function applyRules(name, rules) {
    if (!rules) return name.replace(/\.(service|target)$/, "");
    let n = name.replace(/\.(service|target)$/, "");
    if (rules.strip_prefix && n.startsWith(rules.strip_prefix)) n = n.slice(rules.strip_prefix.length);
    if (rules.strip_suffix && n.endsWith(rules.strip_suffix)) n = n.slice(0, -rules.strip_suffix.length);
    if (rules.aliases && rules.aliases[n]) n = rules.aliases[n];
    return n;
}

function pillsFor(name, entry) {
    const rules = LABEL_RULES[name];
    const pills = [];
    let chart = null;
    let chartPeak = "";
    if (entry.error != null || entry.value == null) return { down: true, pills, chart };

    if (entry.kind === "docker") {
        for (const c of entry.value) pills.push({ label: c.name, cls: classifyDocker(c), tip: c.status });
    } else if (entry.kind === "systemd") {
        for (const u of entry.value) pills.push({ label: applyRules(u.name, rules), cls: classifySystemd(u), tip: u.name });
    } else if (entry.kind === "pihole") {
        const q = entry.value.queries || {}, cl = entry.value.clients || {};
        pills.push({ label: `${Math.round(q.percent_blocked ?? 0)}% blocked`, cls: "ok", tip: `${q.blocked}/${q.total}` });
        pills.push({ label: `${cl.active} clients`, cls: "idle", tip: `${cl.total} known` });
    } else if (entry.kind === "unifi") {
        for (const d of entry.value.devices || []) {
            const cls = d.state === "ONLINE" ? (d.firmware_updatable ? "ghost" : "ok") : "bad";
            pills.push({ label: d.name, cls, tip: `${d.model}${d.firmware_updatable ? " · update available" : ""}` });
        }
        pills.push({ label: `${entry.value.clients} clients`, cls: "idle", tip: "" });
        const g = entry.value.gateway;
        chart = g?.history?.length > 1 ? g.history : null;
        if (chart) {
            const peak = Math.max(...chart.map(b => Math.max(b.tx, b.rx)));
            chartPeak = `peak ${humanBps(peak)}`;
        }
    } else {
        const specs = PILL_SPECS[name];
        if (specs && specs.length) {
            for (const spec of specs) pills.push(pluckPill(entry.value, spec));
        } else {
            pills.push({ label: "up", cls: "idle", tip: `fetched ${entry.fetched_at}` });
        }
    }
    return { down: false, pills, chart, chartPeak }
}

function logSpan(cls, txt) {
    const s = document.createElement("span");
    s.className = cls;
    s.textContent = txt;
    return s;
}

class AttentionBar extends HTMLElement {
    connectedCallback() {
        this._tick();
        this._timer = setInterval(() => this._tick(), 30000);
    }
    disconnectedCallback() { clearInterval(this._timer); }

    async _tick() {
        try {
            const res = await fetch("/api/status");
            this._render(await res.json());
        } catch { /* keep last */ }
    }

    _tabOf(source) {
        for (const t of TABS) {
            if (!t.sources.length || t.sources.includes(source)) return t.name;
        }
        return null;
    }


    _render(status) {
        const attention = [];
        let up = 0, down = 0;

        for (const [name, entry] of Object.entries(status)) {
            const { down: zoneDown, pills } = pillsFor(name, entry);
            const tab = this._tabOf(name);
            if (zoneDown) {
                down++;
                attention.push({ zone: name, tab, label: "source down", cls: "bad", prio: 0 });
            } else {
                up++;
                for (const p of pills) {
                    if (p.cls in ATTENTION) attention.push({ zone: name, tab, label: p.label, cls: p.cls, prio: ATTENTION[p.cls] });
                }
            }
        }
        attention.sort((a, b) => a.prio - b.prio);

        const el = (tag, cls, txt) => { const e = document.createElement(tag); if (cls) e.className = cls;
            if (txt != null) e.textContent = txt; return e; };
        this.replaceChildren();

        const bar = el("div", "strip-totals");
        bar.append(el("span", "t-title", "sources:"), el("span", "t-up", `${up} up`),
            el("span", "t-down", `${down} down`), el("span", "t-total", `${up + down} total`));
        this.appendChild(bar);

        if (!attention.length) return;
        const row = el("div", "attention");
        row.appendChild(el("span", "attn-label", "NEEDS ATTENTION"));
        for (const a of attention) {
            const pill = el("span", "pill " + a.cls, a.label);
            const chip = el("span", "att-item");
            chip.append(pill, el("span", "att-zone", a.zone));
            if (a.tab) {
                const tag = el("span", "att-tab", a.tab);
                tag.addEventListener("click", () => mountTab(TABS.findIndex(t => t.name === a.tab)));
                chip.appendChild(tag);
            }
            row.appendChild(chip);
        }
        this.appendChild(row);
    }
}

customElements.define("attention-bar", AttentionBar);

class StatusStrip extends HTMLElement {
    connectedCallback() {
        this._status = {};
        this._tick();
        this._timer = setInterval(() => this._tick(), 30000);
    }

    async _tick() {
        try {
            const res = await fetch("/api/status");
            this._status = await res.json();
            this.refresh();
        } catch { }
    }

    refresh() {                      // re-render from cache; called on tab switch
        this._render(this._status);
    }

    _render(status) {
        const zones = [];
        const allowed = TABS[ACTIVE]?.sources || [];
        const idx = n => { const i = SOURCE_ORDER.indexOf(n); return i < 0 ? 1e9 : i; };
        const ordered = Object.entries(status)
            .filter(([name]) => !allowed.length || allowed.includes(name))
            .sort((a, b) => idx(a[0]) - idx(b[0]) || a[0].localeCompare(b[0]));
        for (const [name, entry] of ordered) {
            const {down: zoneDown, pills, chart, chartPeak} = pillsFor(name, entry);
            const okCount = pills.filter(p => p.cls === "ok" || p.cls === "idle").length;
            zones.push({name, zoneDown, pills, chart, chartPeak, count: `${okCount}/${pills.length}`});
        }
        this._paint(zones);
    }

    _paint(zones) {
        const el = (tag, cls, txt) => {
            const e = document.createElement(tag);
            if (cls) e.className = cls;
            if (txt != null) e.textContent = txt;
            return e;
        };

        this.replaceChildren();

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
            if (z.chart) {
                const wrap = el("div", "zchart");
                const hdr = el("div", "zchart-hdr");
                hdr.append(el("span", "zchart-title", "WAN 24H"), el("span", "zchart-peak", z.chartPeak));
                wrap.append(hdr, sparkline(z.chart));
                row.appendChild(wrap);
            }
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
        if (kind === "log") { this._connectLog(); return; }

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

    _connectLog() {
        const title = this.getAttribute("title") || "log";
        const filter = this.getAttribute("filter") || "";
        this._seen = new Set();
        this._selected = "";   // "" = all

        this.innerHTML = `
        <div class="bar">
            <span>${title}</span>
            <select class="log-filter"><option value="">all clients</option></select>
            <span class="log-dot"></span>
        </div>
        <div class="log-body"></div>
    `;
        const body = this.querySelector(".log-body");
        const dot = this.querySelector(".log-dot");
        const select = this.querySelector(".log-filter");

        select.addEventListener("change", () => {
            this._selected = select.value;
            for (const line of body.children) {
                line.style.display = (!this._selected || line.dataset.client === this._selected) ? "" : "none";
            }
        });

        const url = "/api/queries/stream" + (filter ? "?" + filter : "");
        this._es = new EventSource(url);
        this._es.onopen = () => dot.classList.add("live");
        this._es.onerror = () => dot.classList.remove("live");
        this._es.onmessage = (e) => {
            let row;
            try { row = JSON.parse(e.data); } catch { return; }

            if (!this._seen.has(row.client)) {
                this._seen.add(row.client);
                const opt = document.createElement("option");
                opt.value = row.client; opt.textContent = row.client;
                select.appendChild(opt);
            }

            const line = document.createElement("div");
            line.className = "log-line log-" + row.status.toLowerCase();
            line.dataset.client = row.client;
            if (this._selected && row.client !== this._selected) line.style.display = "none";
            const t = new Date(row.time * 1000).toLocaleTimeString();
            line.append(
                logSpan("log-time", t),
                logSpan("log-client", row.client),
                logSpan("log-domain", row.domain),
                logSpan("log-status", row.status),
            );
            body.prepend(line);
            while (body.children.length > 200) body.lastChild.remove();
        };
    }

    disconnectedCallback() {
        if (this._es) this._es.close();
        clearTimeout(this._deadline);
    }
}

customElements.define("sitrep-pane", SitrepPane)

const PHONETIC = ["ALPHA", "BRAVO", "CHARLIE", "DELTA", "ECHO", "FOXTROT", "GOLF", "HOTEL", "INDIA"];
let TABS = [];
let ACTIVE = 0;

function normalizeTabs(layout) {
    if (layout?.tabs?.length) {
        return layout.tabs.map((t, i) => ({
            name: t.name || PHONETIC[i] || `TAB ${i + 1}`,
            sources: t.sources || [],
            panes: t.panes || [],
        }));
    }
    return [{ name: null, sources: [], panes: layout?.panes || [] }];
}

function mountTab(i) {
    ACTIVE = i;
    const panesRoot = document.getElementById("panes");
    panesRoot.replaceChildren();
    for (const p of TABS[i].panes) {
        const el = document.createElement("sitrep-pane");
        el.setAttribute("title", p.title);
        el.setAttribute("url", p.url);
        el.setAttribute("kind", p.kind);
        if (p.filter) el.setAttribute("filter", new URLSearchParams(p.filter).toString());
        panesRoot.appendChild(el);
    }
    document.getElementById("strip")?.refresh?.();
    for (const [j, btn] of [...document.getElementById("rail").children].entries()) {
        btn.classList.toggle("active", j === i);
    }
}

function buildRail() {
    const rail = document.getElementById("rail");
    rail.replaceChildren();
    if (TABS.length <= 1) return;
    TABS.forEach((tab, i) => {
        const btn = document.createElement("button");
        btn.type = "button";
        btn.className = "tab" + (i === ACTIVE ? " active" : "");
        btn.textContent = tab.name;
        btn.style.setProperty("--tab-index", String(i));
        btn.addEventListener("click", () => mountTab(i));
        rail.appendChild(btn);
    });
}

function sparkline(history) {
    const W = 600, H = 80, n = history.length;
    const peak = Math.max(1, ...history.map(b => Math.max(b.tx, b.rx)));
    const SLOTS = 288;
    const x = (i) => (i / (SLOTS -1)) * W;
    const y = v => 78 - (v / peak) * 76;   // baseline at 78, peaks to 4 — 2px inset top & bottom
    const pts = key => history.map((b,i) => `${x(i).toFixed(1)},${y(b[key]).toFixed(1)}`).join(" ");

    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("viewBox", `0 0 ${W} ${H}`);
    svg.setAttribute("preserveAspectRatio", "none");
    svg.classList.add("spark");

    for (let h = 1; h < 6; h++) {
        const gx = (h / 6) * W;
        const grid = document.createElementNS("http://www.w3.org/2000/svg", "line");
        grid.setAttribute("x1", gx); grid.setAttribute("x2", gx);
        grid.setAttribute("y1", 4); grid.setAttribute("y2", 76);
        grid.classList.add("spark-grid");
        svg.appendChild(grid);
    }

    for (const [key, cls] of [["rx", "spark-rx"], ["tx", "spark-tx"]]) {
        const line = document.createElementNS("http://www.w3.org/2000/svg", "polyline");
        line.setAttribute("points", pts(key));
        line.classList.add(cls);
        svg.appendChild(line);
    }
    const latest = history[n - 1];
    svg.setAttribute("aria-label", `WAN ${humanBps(latest.rx)} down / ${humanBps(latest.tx)} up`);
    return svg;
}

function humanBps(bps) {
    const bits = bps * 8;
    if (bits >= 1e9) return `${(bits / 1e9).toFixed(1)} Gbps`;
    if (bits >= 1e6) return `${(bits / 1e6).toFixed(1)} Mbps`;
    if (bits >= 1e3) return `${(bits / 1e3).toFixed(0)} Kbps`;
    return `${bits} bps`;
}

async function boot() {
    const cfg = await (await fetch("/api/config")).json();
    LABEL_RULES = cfg.label_rules || {};
    PILL_SPECS = cfg.pill_specs || {};
    SOURCE_ORDER = cfg.source_order || [];

    for (const [key, value] of Object.entries(cfg.theme || {})) {
        document.documentElement.style.setProperty(`--${key}`, value);
    }

    TABS = normalizeTabs(cfg.layout);
    buildRail();
    mountTab(0);

    document.addEventListener("keydown", (e) => {
        if (e.target instanceof HTMLInputElement) return;
        const n = parseInt(e.key, 10);
        if (n >= 1 && n <= TABS.length) mountTab(n - 1);
    });
}

boot();