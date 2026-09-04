import { test } from "node:test";
import assert from "node:assert/strict";
import { classifySystemd, classifyDocker, shortLabel } from "../static/classify.js";

const unit = (over) => ({ name: "x.service", active: "inactive", sub: "dead", type: "oneshot", result: "success", ...over });

test("systemd classifier", () => {
    const cases = [
        ["failed unit",                  unit({ active: "failed" }),                              "bad"],
        ["non-success result",           unit({ result: "exit-code" }),                           "bad"],
        ["activating",                   unit({ active: "activating" }),                          "run"],
        ["deactivating",                 unit({ active: "deactivating" }),                        "run"],
        ["dangling service (empty type)",unit({ type: "", result: "" }),                          "ghost"],
        ["running service",              unit({ active: "active", type: "simple" }),              "ok"],
        ["idle-clean oneshot",           unit({}),                                                "idle"],
        ["target with empty result",     unit({ name: "t.target", type: "", result: "" }),        "idle"],
        ["unknown combination",          unit({ active: "reloading", result: "" }),               "ghost"],
    ];
    for (const [desc, input, want] of cases) {
        assert.equal(classifySystemd(input), want, desc);
    }
});


test("docker classifier", () => {
    const c = (over) => ({ name: "c", state: "running", status: "Up", health: "healthy", ...over });
    const cases = [
        ["running healthy",        c({}),                          "ok"],
        ["running no healthcheck", c({ health: "none" }),          "ok"],
        ["running health absent",  c({ health: null }),            "ok"],
        ["running unhealthy",      c({ health: "unhealthy" }),     "bad"],
        ["running starting",       c({ health: "starting" }),      "run"],
        ["exited",                 c({ state: "exited" }),         "bad"],
        ["restarting",             c({ state: "restarting" }),     "run"],
        ["created",                c({ state: "created" }),        "run"],
        ["paused",                 c({ state: "paused" }),         "ghost"],
    ];
    for (const [desc, input, want] of cases) {
        assert.equal(classifyDocker(input), want, desc);
    }
});

test("shortLabel strips unit suffixes only", () => {
    assert.equal(shortLabel("ingest-era5.service"), "ingest-era5");
    assert.equal(shortLabel("ingest-daily.target"), "ingest-daily");
    assert.equal(shortLabel("plain-name"), "plain-name");
    assert.equal(shortLabel("service.target.service"), "service.target");
});