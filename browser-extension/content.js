let currentUrl = null;

function bestImageSource(element) {
  if (!element) return null;
  if (element.currentSrc) return element.currentSrc;
  if (element.src) return element.src;
  const link = element.closest && element.closest("a[href]");
  if (link && /\.(avif|bmp|gif|heic|heif|ico|jpe?g|jfif|png|svg|tiff?|webp)(\?|#|$)/i.test(link.href)) {
    return link.href;
  }
  return null;
}

document.addEventListener("mouseover", (event) => {
  const target = event.target;
  const url = target instanceof HTMLImageElement ? bestImageSource(target) : bestImageSource(target && target.closest && target.closest("a[href]"));
  if (!url || url === currentUrl) return;
  currentUrl = url;
  chrome.runtime.sendMessage({ type: "hover", url });
}, { passive: true });

document.addEventListener("mouseout", (event) => {
  if (!currentUrl) return;
  const related = event.relatedTarget;
  if (related && event.target && event.target.contains && event.target.contains(related)) return;
  currentUrl = null;
  chrome.runtime.sendMessage({ type: "exit" });
}, { passive: true });