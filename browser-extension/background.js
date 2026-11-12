const HOST = "com.hover.preview";

chrome.runtime.onMessage.addListener((message) => {
  if (!message || message.type !== "hover" && message.type !== "exit") {
    return;
  }
  const port = chrome.runtime.connectNative(HOST);
  port.postMessage(message);
  port.disconnect();
});