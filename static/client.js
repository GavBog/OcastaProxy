const path = window.location.href.split("/").slice(4).join("/");
const { hash, host, hostname, href, origin, pathname, port, protocol, search } =
  new URL(path);

window.$Ocasta = {};
window.$Ocasta.location = {
  ...window.location,
  hash,
  host,
  hostname,
  href,
  origin,
  pathname,
  port,
  protocol,
  search,
};

// const encoding = window.location.pathname.split("/")[1];

// function rewriteURL(url) {
//   url = url.toString();
//   if (url.startsWith(`${window.location.origin}/${encoding}/`)) {
//     return url;
//   }
//   if (url.match(/^(data:|about:|javascript:|blob:|mailto:)/g)) {
//     return url;
//   }
//   if (url.startsWith(window.location.origin)) {
//     url = url.replace(window.location.origin, "");
//   }
//   if (url.startsWith("./")) {
//     url = url.splice(2);
//   }
//   const validProtocol = url.startsWith("http://") || url.startsWith("https://");
//   if (
//     !origin.endsWith("/") &&
//     !url.startsWith("/") &&
//     !url.startsWith("http:") &&
//     !url.startsWith("https:")
//   ) {
//     url = "/" + url;
//   }
//   if (!validProtocol) {
//     url = origin + url;
//   }
//   if (encoding === "b64") {
//     url = btoa(url);
//   }

//   url = `/${encoding}/${url}`;
//   return url;
// }

// const observer = new MutationObserver((mutations) => {
//   mutations.forEach((mutation) => {
//     if (mutation.type === "childList") {
//       mutation.addedNodes.forEach((node) => {
//         if (node.src) {
//           node.src = rewriteURL(node.src);

//           // Incredibly hacky way to load scripts
//           if (node.tagName === "SCRIPT") {
//             fetch(node.src)
//               .then((res) => res.text())
//               .then((text) => {
//                 eval(text);
//               });
//           }
//         }
//         if (node.href) {
//           node.href = rewriteURL(node.href);
//           // Incredibly hacky way to load scripts
//           if (node.tagName === "LINK" && node.as === "script") {
//             fetch(node.href)
//               .then((res) => res.text())
//               .then((text) => {
//                 eval(text);
//               });
//           }
//         }
//       });
//     }
//   });
// });

// observer.observe(document, {
//   childList: true,
//   subtree: true,
// });
