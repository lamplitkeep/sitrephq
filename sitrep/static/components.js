// @ts-check
import { classifySystemd, classifyDocker, shortLabel } from "./classify.js?v=1";

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

        }
    }

    _render(status) {
        const zones = [];
        for (const [name, entry] of Object.entries(status).sort()) {
            const pills = [];
            let down = false;

            if (entry.error != null || entry.value == null) {
                down = true;
            } else if (entry.kind === "docker") {
                for (const c of entry.value) {
                    pills.push({label: c.name, cls: classifyDocker(c), tip: c.status});
                }
            } else if (entry.kind === "systemd") {
                for (const u of entry.value) {
                    pills.push({label: shortLabel(u.name), cls: classifySystemd(u), tip: u.name});
                }
            } else if (entry.kind === "pihole") {
                const q = entry.value.queries || {};
                const c = entry.value.clients || {};
                pills.push({ label: `${Math.round(q.percent_blocked ?? 0)}% blocked`, cls: "ok", tip: `${q.blocked} of ${q.total} queries` });
                pills.push({ label: `${c.active} clients`, cls: "idle", tip: `${c.total} known` });
            } else {
                pills.push({ label: "up", cls: "idle", tip: `fetched ${entry.fetched_at}` });
            }

            zones.push({ name, down, pills, tip: entry.error || "" });
        }

        this.replaceChildren(...zones.map(z => {
            const zone = document.createElement("div");
            zone.className = "zone" + (z.down ? " down" : "");
            zone.title = z.tip;

            const label = document.createElement("span");
            label.className = "zlabel";
            label.textContent = z.name;
            zone.appendChild(label);

            for (const p of z.pills) {
                const pill = document.createElement("span");
                pill.className = "pill " + p.cls;
                pill.textContent = p.label;
                pill.title = p.tip;
                zone.appendChild(pill);
            }

            return zone;
        }));
    }
}

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
customElements.define("status-strip", StatusStrip);

async function boot() {
    const cfg = await (await fetch("/api/config")).json();
    const root = document.getElementById("panes");
    for (const [key, value] of Object.entries(cfg.theme || {})) {
        document.documentElement.style.setProperty(`--${key}`, value);
    }
    for (const p of cfg.layout?.panes || []) {
        const el = document.createElement("sitrep-pane");
        el.setAttribute("title", p.title);
        el.setAttribute("url", p.url);
        el.setAttribute("kind", p.kind);
        root.appendChild(el);
    }
}

boot();