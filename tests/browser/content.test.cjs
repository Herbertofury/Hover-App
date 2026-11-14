const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');

function fixture() {
  const messages = [];
  function surface() {
    const handlers = new Map();
    return {
      hidden: false,
      addEventListener(type, handler) { handlers.set(type, handler); },
      fire(type, event = {}) { handlers.get(type)?.(event); },
    };
  }
  const document = surface(), window = surface();
  class HTMLImageElement {
    constructor() { this.currentSrc = 'https://example.org/image.png'; }
    contains(other) { return this === other; }
  }
  const context = {document, window, HTMLImageElement,
    chrome: {runtime: {sendMessage: message => messages.push(message)}}};
  vm.runInNewContext(fs.readFileSync(path.join(__dirname, '../../browser-extension/content.js'), 'utf8'), context);
  const image = new HTMLImageElement();
  return {document, window, messages, image, hover: () => document.fire('mouseover', {target: image})};
}

for (const event of ['blur', 'pagehide']) {
  test(`${event} exits once and permits the same target after returning`, () => {
    const f = fixture();
    f.hover(); f.window.fire(event); f.window.fire(event);
    assert.equal(f.messages.map(m => m.type).join(','), 'hover,exit');
    f.hover();
    assert.equal(f.messages.map(m => m.type).join(','), 'hover,exit,hover');
  });
}

test('hidden document exits; visible transition alone does not activate', () => {
  const f = fixture(); f.hover();
  f.document.hidden = true; f.document.fire('visibilitychange');
  f.document.hidden = false; f.document.fire('visibilitychange');
  assert.equal(f.messages.map(m => m.type).join(','), 'hover,exit');
  f.hover(); assert.equal(f.messages.length, 3);
});

test('no target means no unnecessary exit', () => {
  const f = fixture();
  f.window.fire('blur'); f.window.fire('pagehide');
  f.document.hidden = true; f.document.fire('visibilitychange');
  assert.equal(f.messages.length, 0);
});

test('same target and internal movement preserve the active hover', () => {
  const f = fixture(); f.hover(); f.hover();
  f.document.fire('mouseout', {target: f.image, relatedTarget: f.image});
  assert.equal(f.messages.length, 1);
  f.document.fire('mouseout', {target: f.image});
  assert.equal(f.messages.map(m => m.type).join(','), 'hover,exit');
});
