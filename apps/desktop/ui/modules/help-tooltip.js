export function setHelpTooltip(helpElement, text, { hideWhenEmpty = false } = {}) {
  if (!helpElement) {
    return;
  }
  const message = String(text || '').trim();
  if (!message) {
    helpElement.dataset.tooltip = '';
    if (hideWhenEmpty) {
      helpElement.hidden = true;
    }
    return;
  }
  helpElement.dataset.tooltip = message;
  helpElement.hidden = false;
}
