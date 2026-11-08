// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  activate,
  focusFirstIn,
  focusRoot,
  focusables,
  goBack,
  moveFocus,
  resetFocusMemory,
  switchSection,
} from "./navigation";

const visible = () => true;

// Rect factice : lit data-r="left,top,right,bottom".
function fakeRect(el: HTMLElement) {
  const [left, top, right, bottom] = (el.dataset.r ?? "0,0,0,0").split(",").map(Number);
  return { left, top, right, bottom };
}
const opts = { isVisible: visible, rect: fakeRect };

afterEach(() => {
  resetFocusMemory();
  document.body.innerHTML = "";
  (document.activeElement as HTMLElement | null)?.blur?.();
});

describe("focusRoot", () => {
  it("returns body without dialog", () => {
    expect(focusRoot()).toBe(document.body);
  });
  it("returns the last modal dialog", () => {
    document.body.innerHTML = `<div id="d1" role="dialog" aria-modal="true"></div><div id="d2" role="dialog" aria-modal="true"></div>`;
    expect(focusRoot().id).toBe("d2");
  });
});

describe("focusables", () => {
  it("skips disabled and tabindex -1", () => {
    document.body.innerHTML = `<button id="b1"></button><button id="b2" disabled></button><button id="b3" tabindex="-1"></button><a id="a" href="#x">x</a>`;
    expect(focusables(document.body, visible).map((e) => e.id)).toEqual(["b1", "a"]);
  });
});

describe("moveFocus", () => {
  it("focuses first when nothing is focused", () => {
    document.body.innerHTML = `<button id="b1" data-r="0,0,10,10"></button><button id="b2" data-r="20,0,30,10"></button>`;
    moveFocus("right", opts);
    expect(document.activeElement?.id).toBe("b1");
  });
  it("moves spatially", () => {
    document.body.innerHTML = `<button id="b1" data-r="0,0,10,10"></button><button id="b2" data-r="20,0,30,10"></button>`;
    document.getElementById("b1")!.focus();
    moveFocus("right", opts);
    expect(document.activeElement?.id).toBe("b2");
  });
  it("stays inside the dialog", () => {
    document.body.innerHTML = `<button id="out" data-r="20,0,30,10"></button>
      <div role="dialog" aria-modal="true"><button id="in" data-r="0,0,10,10"></button></div>`;
    document.getElementById("in")!.focus();
    moveFocus("right", opts);
    expect(document.activeElement?.id).toBe("in");
  });
  it("select left/right changes option", () => {
    document.body.innerHTML = `<select id="s"><option>a</option><option>b</option><option>c</option></select>`;
    const s = document.getElementById("s") as HTMLSelectElement;
    s.focus();
    const onChange = vi.fn();
    document.body.addEventListener("change", onChange);
    moveFocus("right", opts);
    expect(s.selectedIndex).toBe(1);
    expect(onChange).toHaveBeenCalledTimes(1);
    moveFocus("left", opts);
    moveFocus("left", opts);
    expect(s.selectedIndex).toBe(0);
  });
  it("select ignores repeated moves", () => {
    document.body.innerHTML = `<select id="s"><option>a</option><option>b</option><option>c</option></select>`;
    const s = document.getElementById("s") as HTMLSelectElement;
    s.focus();
    moveFocus("right", { ...opts, repeat: true });
    moveFocus("right", { ...opts, repeat: true });
    expect(s.selectedIndex).toBe(0);
    expect(document.activeElement).toBe(s);
  });
  it("data-pad-keys on the focused element forwards arrows", () => {
    document.body.innerHTML = `<div id="c" data-r="0,0,50,10"><button id="b" data-pad-keys data-r="0,0,10,10"></button><button id="m" data-r="20,0,30,10"></button></div><button id="n" data-r="0,20,10,30"></button>`;
    const c = document.getElementById("c")!;
    const onKey = vi.fn();
    c.addEventListener("keydown", onKey);
    document.getElementById("m")!.focus();
    moveFocus("left", opts);
    expect(onKey).not.toHaveBeenCalled();
    expect(document.activeElement?.id).toBe("b");
    moveFocus("right", opts);
    expect(onKey).toHaveBeenCalledTimes(1);
    expect(document.activeElement?.id).toBe("b");
    moveFocus("down", opts);
    expect(document.activeElement?.id).toBe("n");
  });
  it("forwards the arrow key name", () => {
    document.body.innerHTML = `<div id="c" data-r="0,0,50,10"><button id="b" data-pad-keys data-r="0,0,10,10"></button></div><button id="n" data-r="0,20,10,30"></button>`;
    const c = document.getElementById("c")!;
    const onKey = vi.fn();
    c.addEventListener("keydown", onKey);
    document.getElementById("b")!.focus();
    moveFocus("left", opts);
    expect(onKey).toHaveBeenCalledTimes(1);
    expect((onKey.mock.calls[0][0] as KeyboardEvent).key).toBe("ArrowLeft");
    expect(document.activeElement?.id).toBe("b");
    moveFocus("down", opts);
    expect(document.activeElement?.id).toBe("n");
  });
});

describe("activate", () => {
  it("clicks a button", () => {
    document.body.innerHTML = `<button id="b"></button>`;
    const b = document.getElementById("b")!;
    const onClick = vi.fn();
    b.addEventListener("click", onClick);
    b.focus();
    activate();
    expect(onClick).toHaveBeenCalledTimes(1);
  });
  it("checks a checkbox", () => {
    document.body.innerHTML = `<input id="c" type="checkbox" />`;
    const c = document.getElementById("c") as HTMLInputElement;
    c.focus();
    activate();
    expect(c.checked).toBe(true);
  });
  it("does not click a text field", () => {
    document.body.innerHTML = `<input id="t" type="text" />`;
    const t = document.getElementById("t")!;
    const onClick = vi.fn();
    t.addEventListener("click", onClick);
    t.focus();
    activate();
    expect(onClick).not.toHaveBeenCalled();
  });
});

describe("goBack", () => {
  it("sends Escape when a dialog is open", () => {
    document.body.innerHTML = `<div role="dialog" aria-modal="true"></div>`;
    const onKey = vi.fn();
    window.addEventListener("keydown", onKey);
    const nav = vi.fn();
    goBack(nav, "/library");
    window.removeEventListener("keydown", onKey);
    expect(onKey).toHaveBeenCalledTimes(1);
    expect((onKey.mock.calls[0][0] as KeyboardEvent).key).toBe("Escape");
    expect(nav).not.toHaveBeenCalled();
  });
  it("navigates back outside home", () => {
    const nav = vi.fn();
    goBack(nav, "/library");
    expect(nav).toHaveBeenCalledWith(-1);
  });
  it("does nothing on home", () => {
    const nav = vi.fn();
    goBack(nav, "/");
    expect(nav).not.toHaveBeenCalled();
  });
});

describe("switchSection", () => {
  const html = `<aside class="sidebar"><nav class="nav"><a href="#/a">a</a><a id="l2" href="#/b" class="active">b</a><a id="l3" href="#/c">c</a></nav></aside>`;
  it("clicks the next link", () => {
    document.body.innerHTML = html;
    const onClick = vi.fn((e: Event) => e.preventDefault());
    document.getElementById("l3")!.addEventListener("click", onClick);
    expect(switchSection(1)).toBe(true);
    expect(onClick).toHaveBeenCalledTimes(1);
  });
  it("returns false at the boundary", () => {
    document.body.innerHTML = html;
    document.getElementById("l2")!.classList.remove("active");
    document.getElementById("l3")!.classList.add("active");
    expect(switchSection(1)).toBe(false);
  });
});

describe("moveFocus dans un dialogue", () => {
  it("focuses the first button when the dialog itself is focused", () => {
    document.body.innerHTML = `<button id="out" data-r="0,0,10,10"></button><div id="dlg" role="dialog" aria-modal="true" tabindex="-1" data-r="0,0,100,100"><button id="b1" data-r="10,10,40,30"></button><button id="b2" data-r="10,50,40,70"></button></div>`;
    document.getElementById("dlg")!.focus();
    moveFocus("down", opts);
    expect(document.activeElement?.id).toBe("b1");
  });
});

describe("focusFirstIn", () => {
  it("focuses the first focusable in the matching element", () => {
    document.body.innerHTML = `<div class="x"><button id="b1"></button><button id="b2"></button></div>`;
    focusFirstIn(".x", visible);
    expect(document.activeElement?.id).toBe("b1");
  });
  it("does nothing when nothing matches", () => {
    document.body.innerHTML = `<button id="b1"></button>`;
    focusFirstIn(".nope");
    expect(document.activeElement).toBe(document.body);
  });
});

describe("repli du focus", () => {
  it("focused element removed -> nearest remaining element, not the first", () => {
    document.body.innerHTML = `<main><button id="a" data-r="0,0,10,10"></button><button id="b" data-r="0,100,10,110"></button><button id="c" data-r="0,200,10,210"></button><button id="d" data-r="0,300,10,310"></button></main>`;
    document.getElementById("c")!.focus();
    moveFocus("left", opts); // mémorise c
    document.getElementById("c")!.remove();
    moveFocus("down", opts);
    expect(["b", "d"]).toContain(document.activeElement?.id);
  });
  it("remembered element still connected wins", () => {
    document.body.innerHTML = `<main><button id="a" data-r="0,0,10,10"></button><button id="b" data-r="0,100,10,110"></button></main>`;
    document.getElementById("b")!.focus();
    moveFocus("left", opts);
    (document.activeElement as HTMLElement).blur();
    moveFocus("left", opts);
    expect(document.activeElement?.id).toBe("b");
  });
  it("dialog closed via goBack -> opener focused again", () => {
    document.body.innerHTML = `<main><button id="a" data-r="0,0,10,10"></button><button id="op" data-r="0,100,10,110"></button></main>`;
    document.getElementById("op")!.focus();
    activate();
    document.body.insertAdjacentHTML("beforeend", `<div id="dlg" role="dialog" aria-modal="true" tabindex="-1" data-r="0,0,100,100"><header><button id="x" data-r="0,0,5,5"></button></header><div class="content"><button id="in" data-r="10,10,20,20"></button></div></div>`);
    document.getElementById("dlg")!.focus();
    moveFocus("down", opts);
    expect(document.activeElement?.id).toBe("in");
    goBack(vi.fn(), "/library");
    document.getElementById("dlg")!.remove();
    moveFocus("down", opts);
    expect(document.activeElement?.id).toBe("op");
  });
  it("first move in a fresh dialog skips the header close button", () => {
    document.body.innerHTML = `<div id="dlg" role="dialog" aria-modal="true" tabindex="-1"><header><button id="x"></button></header><div class="content"><button id="in"></button></div></div>`;
    document.getElementById("dlg")!.focus();
    moveFocus("down", opts);
    expect(document.activeElement?.id).toBe("in");
  });
});

describe("switchSection et dialogue", () => {
  it("returns false and clicks nothing while a dialog is open", () => {
    document.body.innerHTML = `<aside class="sidebar"><nav class="nav"><a href="#/a" class="active">a</a><a id="l2" href="#/b">b</a></nav></aside><div role="dialog" aria-modal="true"></div>`;
    const onClick = vi.fn((e: Event) => e.preventDefault());
    document.getElementById("l2")!.addEventListener("click", onClick);
    expect(switchSection(1)).toBe(false);
    expect(onClick).not.toHaveBeenCalled();
  });
});

describe("focusFirstIn et data-pad-autofocus", () => {
  it("prefers the marked element", () => {
    document.body.innerHTML = `<div class="x"><button id="b1"></button><button id="b2" data-pad-autofocus></button></div>`;
    focusFirstIn(".x", visible);
    expect(document.activeElement?.id).toBe("b2");
  });
});
