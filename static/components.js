// @ts-check

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
                <iframe src="${url}" loading="lazy" referrerpolicy="no-referrer">
                <div class="veil">pane block or unreachable &dash; check VPN, DNS & targets frame headers</div>
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
                const render = parseFloat(getComputedStyle(this).getPropertyValue("--pane-renderer-width")) || 1280;
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
    const res = await fetch("/api/config");
    const cfg = await res.json();
    const root = document.getElementById("panes");
    for (const p of cfg.panes || []) {
        const el = document.createElement("sitrep-pane");
        el.setAttribute("title", p.title);
        el.setAttribute("url", p.url);
        el.setAttribute("kind", p.kind);
        root.appendChild(el);
    }
}

boot();