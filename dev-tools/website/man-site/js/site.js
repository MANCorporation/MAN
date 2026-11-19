// MAN website: bigger text switch, and the app list on apps.html.
(function () {
  "use strict";

  // "A+": bigger text, remembered on this computer.
  var root = document.documentElement;
  try { if (localStorage.getItem("man-big-text") === "1") root.classList.add("big-text"); } catch (e) {}
  document.addEventListener("DOMContentLoaded", function () {
    var b = document.getElementById("textsize");
    if (b) {
      b.setAttribute("aria-pressed", root.classList.contains("big-text") ? "true" : "false");
      b.addEventListener("click", function () {
        var on = root.classList.toggle("big-text");
        b.setAttribute("aria-pressed", on ? "true" : "false");
        try { localStorage.setItem("man-big-text", on ? "1" : "0"); } catch (e) {}
      });
    }
    if (document.getElementById("applist")) apps();
  });

  function apps() {
    var list = document.getElementById("applist");
    var search = document.getElementById("appsearch");
    var more = document.getElementById("appmore");
    var count = document.getElementById("appcount");
    var cats = document.querySelectorAll("#appcats button");
    var all = [], shown = 0, matches = [], cat = "", PAGE = 48;

    function esc(s) { return String(s).replace(/[&<>"]/g, function (c) { return { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]; }); }
    function render(reset) {
      if (reset) { list.textContent = ""; shown = 0; }
      var html = "";
      matches.slice(shown, shown + PAGE).forEach(function (a) {
        html += '<div class="card app"><img loading="lazy" src="' + esc(a.icon) + '" alt="" width="48" height="48">' +
          '<div><div class="name">' + esc(a.name) + '</div><div class="sum">' + esc(a.summary) + "</div></div></div>";
      });
      list.insertAdjacentHTML("beforeend", html);
      shown = Math.min(matches.length, shown + PAGE);
      more.hidden = shown >= matches.length;
      count.textContent = matches.length === 1 ? "1 app" : matches.length.toLocaleString() + " apps";
    }
    function filter() {
      var q = search.value.trim().toLowerCase();
      matches = all.filter(function (a) {
        if (cat && a.cat !== cat) return false;
        if (!q) return true;
        return (a.name + " " + a.summary + " " + (a.by || "")).toLowerCase().indexOf(q) >= 0;
      });
      render(true);
    }
    cats.forEach(function (b) {
      b.addEventListener("click", function () {
        cats.forEach(function (x) { x.setAttribute("aria-pressed", "false"); });
        b.setAttribute("aria-pressed", "true");
        cat = b.getAttribute("data-cat");
        filter();
      });
    });
    search.addEventListener("input", filter);
    more.addEventListener("click", function () { render(false); });

    fetch("data/apps.json").then(function (r) { return r.json(); }).then(function (data) {
      all = data.sort(function (a, b) { return (b.n || 0) - (a.n || 0); });
      filter();
    }).catch(function () {
      count.textContent = "The app list could not be loaded. Open Software on MAN to see all apps.";
    });
  }
})();
