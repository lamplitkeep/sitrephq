// @ts-check

export function classifySystemd(u) {
    if (u.active === "failed") return "bad";
    if (u.result && u.result !== "success" && u.result !== "") return "bad";
    if (u.active === "activating" || u.active === "deactivating") return "run";
    if (u.type === "" && u.name.endsWith(".service")) return "ghost";
    if (u.active === "active") return "ok";
    if (u.active === "inactive" && u.result === "success") return "idle";
    if (u.name.endsWith(".target")) return "idle";
    return "ghost";
}

export function classifyDocker(c) {
    if (c.state === "running") {
        if (c.health === "unhealthy") return "bad";
        if (c.health === "starting") return "run";
        return "ok";
    }
    if (c.state === "restarting" || c.state === "created") return "run";
    if (c.state === "exited") return "bad";
    return "ghost";
}

export function shortLabel(name) {
    return name.replace(/\.(service|target)$/, "");
}