// jsdom tidak mengimplementasikan API layout yang dipakai Reka Select.
if (typeof Element !== 'undefined' && !Element.prototype.scrollIntoView) {
  Element.prototype.scrollIntoView = () => {}
}
