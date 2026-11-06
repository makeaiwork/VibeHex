// Local stand-in for the design canvas runtime, so a .dc.html mockup opened from disk renders as it
// does on the canvas: <helmet> goes to <head>, {{holes}}, <sc-for>, <sc-if>, onClick and setState
// work, and canvas image uploads (/_blob/<id>) point at the project's own image files.
// Each mockup folder holds a copy of this file as ./support.js; edit docs/design/support.js and copy.
// Per project: set ASSETS (path from a mockup folder to the project images) and BLOBS (canvas image id -> path).
(() => {
  const ASSETS = '../../../';
  const BLOBS = {
    // '<32-hex canvas image id>': ASSETS + '<file>',  or  '../img/<file>' for an image downloaded from the canvas
    'a59ce7fd973d76cb3a7e5d90303892fd': ASSETS + 'packaging/linux/icons/hicolor/128x128/apps/anomaly.hex.png',
  };
  const fixBlobs = (text) => text.replace(/\/_blob\/([a-f0-9]{32})/g, (whole, id) => BLOBS[id] ?? whole);

  class DCLogic {
    constructor(props) {
      this.props = props ?? {};
      this.state = {};
    }
    setState(patch) {
      Object.assign(this.state, typeof patch === 'function' ? patch(this.state) : patch);
      this.__draw?.();
    }
  }
  window.DCLogic = DCLogic;

  // The rest of the file arrives as raw text, so <sc-for> inside a <table> is not moved by the parser.
  document.write('<plaintext hidden>');

  const lookup = (scope, path) => path.split('.').reduce((value, key) => (value == null ? undefined : value[key]), scope);
  const hole = (expr, scope) => (expr === 'true' ? true : expr === 'false' ? false : lookup(scope, expr));
  const fill = (text, scope) => {
    const whole = text.match(/^\{\{\s*([^}]+?)\s*\}\}$/);
    if (whole) return hole(whole[1], scope);
    return text.replace(/\{\{\s*([^}]+?)\s*\}\}/g, (_, expr) => String(hole(expr, scope) ?? ''));
  };

  function render(node, scope, parent) {
    if (node.nodeType === Node.TEXT_NODE) {
      parent.append(document.createTextNode(String(fill(node.nodeValue, scope) ?? '')));
      return;
    }
    if (node.nodeType !== Node.ELEMENT_NODE) return;
    const kind = node.localName === 'template' ? node.dataset.sc : null;
    if (kind === 'for') {
      const as = node.getAttribute('as') || 'item';
      for (const item of fill(node.getAttribute('list') ?? '', scope) ?? []) {
        const inner = Object.create(scope);
        inner[as] = item;
        for (const child of node.content.childNodes) render(child, inner, parent);
      }
      return;
    }
    if (kind === 'if') {
      if (fill(node.getAttribute('value') ?? '', scope)) for (const child of node.content.childNodes) render(child, scope, parent);
      return;
    }
    const el = document.importNode(node, false);
    for (const { name, value } of [...node.attributes]) {
      if (name.startsWith('hint-')) { el.removeAttribute(name); continue; }
      if (!value.includes('{{')) continue;
      const result = fill(value, scope);
      if (name.startsWith('on') && typeof result === 'function') {
        el.removeAttribute(name);
        el.addEventListener(name.slice(2), result);
      } else if (result === false || result == null) {
        el.removeAttribute(name);
      } else {
        el.setAttribute(name, result === true ? '' : String(result));
      }
    }
    const children = node.localName === 'template' ? node.content.childNodes : node.childNodes;
    for (const child of children) render(child, scope, el);
    parent.append(el);
  }

  document.addEventListener('DOMContentLoaded', () => {
    const raw = document.querySelector('plaintext');
    const source = fixBlobs(raw.textContent)
      .replace(/<sc-(for|if)\b/g, '<template data-sc="$1"')
      .replace(/<\/sc-(?:for|if)>/g, '</template>');
    raw.remove();
    const parsed = document.createElement('template');
    parsed.innerHTML = source.replace(/^[\s\S]*?<body[^>]*>/i, '').replace(/<\/body>[\s\S]*$/i, '');
    const root = parsed.content.querySelector('x-dc');
    const script = parsed.content.querySelector('script[data-dc-script]');
    if (!root) return;
    const helmet = root.querySelector('helmet');
    if (helmet) {
      document.head.append(...[...helmet.childNodes].map((n) => document.importNode(n, true)));
      helmet.remove();
    }

    const props = {};
    let Component = class extends DCLogic { renderVals() { return {}; } };
    if (script) {
      for (const [key, spec] of Object.entries(JSON.parse(script.getAttribute('data-props') || '{}'))) {
        if (!key.startsWith('$') && spec && 'default' in spec) props[key] = spec.default;
      }
      Component = new Function('DCLogic', `${script.textContent}\nreturn Component;`)(DCLogic);
    }
    const view = new Component(props);
    view.__draw = () => {
      const focused = document.activeElement?.id;
      const out = document.createDocumentFragment();
      const scope = view.renderVals?.() ?? {};
      for (const child of root.childNodes) render(child, scope, out);
      document.body.replaceChildren(out);
      if (focused) document.getElementById(focused)?.focus();
    };
    view.__draw();
    view.componentDidMount?.();
  });
})();
