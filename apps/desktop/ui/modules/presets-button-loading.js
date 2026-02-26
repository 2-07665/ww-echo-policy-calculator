export async function withButtonLoading(button, loadingText, run) {
  if (button?.dataset?.loading === 'true') {
    return false;
  }

  const originalText = button.textContent;
  button.dataset.loading = 'true';
  button.disabled = true;
  button.textContent = loadingText;

  try {
    await run();
    return true;
  } finally {
    button.dataset.loading = 'false';
    button.textContent = originalText;
  }
}
