import { test } from "node:test";
import assert from "node:assert/strict";
import { pluck, pluckPill } from "../sitrep/static/pluck.js";

const data = {
    queries: { total: 96246, perent_blocked: 23.9 },
    clients: { active: 37 },
    Datapoints: [ { Average: 100 }, { Average: 200 }, { Average: 300 } ],
    status: "warning",
};

test("plick reaches nested keys", () => {
    assert.equal(pluck(data, "queries.total"), 96246);
    assert.equal(pluck(data, "clients.active"), 37);
});

test("pluck array positive and negative index", () => {
    assert.equal(pluck(data, "Datapoints.0.Average"), 100);
    assert.equal(pluck(data, "Datapoints.-1.Average"), 300);
    assert.equal(pluck(data, "Datapoints.-2.Average"), 200);
});

test("pluck misses return undefined, never throw", () => {
    assert.equal(pluck(data, "queries.nonexistent"), undefined);
    assert.equal(pluck(data, "queries.total.deeper"), undefined); // descend into scalar
    assert.equal(pluck(data, "Datapoints.9.Average"), undefined); // index oob
    assert.equal(pluck(data, "Datapoints.-9.Average"), undefined) // neg index oob
    assert.equal(pluck(data, "totally.absent.path"), undefined);
});

test("pluckPill templates label and shows value", () => {
    const p = pluckPill(data, { path: "clients.active", label: "{} clients", state: "idle"});
    assert.equal(p.label, "37 clients");
    assert.equal(p.cls, "idle");
});

test("pluckPill maps string value to state", () => {
    const p = pluckPill(data, { path: "status", map: { ok: "ok", warning: "ghost", error: "bad" } });
    assert.equal(p.label, "warning");
    assert.equal(p.cls, "ghost");
});

test("pluckPill unmapped value defaults ghost", () => {
    const p = pluckPill(data, { path: "status", map: { ok: "ok" } });
    assert.equal(p.cls, "ghost");
});

test("pluckPill missing value shows ? and ghost", () => {
    const p = pluckPill(data, { path: "not.here", label: "X {}", state: "ok" });
    assert.equal(p.label, "X ?");
    assert.equal(p.cls, "ghost"); // missing forces ghost
});