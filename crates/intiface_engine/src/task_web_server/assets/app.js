// Intiface Engine task diagnostics UI.
//
// Connects to the same-origin SSE stream, maintains an active-task map keyed
// by id, derives a slash-path hierarchy, and keeps a bounded log of endings
// observed during this browser session. Makes no external network requests.

"use strict";

var MAX_LOG_ENTRIES = 200;

(function () {
  var tree = document.getElementById("task-tree");
  var treeEmpty = document.getElementById("tree-empty");
  var logEl = document.getElementById("event-log");
  var connState = document.getElementById("connection-state");
  var activeCount = document.getElementById("active-count");
  var rootCount = document.getElementById("root-count");
  var detachedCount = document.getElementById("detached-count");
  var expandBtn = document.getElementById("expand-all");
  var clearBtn = document.getElementById("clear-log");
  var leafTemplate = document.getElementById("leaf-template");
  var branchTemplate = document.getElementById("branch-template");
  var logRowTemplate = document.getElementById("log-row-template");

  // Active tasks keyed by numeric id (stringified for DOM attribute use).
  var active = Object.create(null);
  // Ordered list of log rows, newest first.
  var logRows = [];

  function setConnectionState(label, cls) {
    connState.textContent = label;
    connState.className = "state " + cls;
  }

  function sortTasks() {
    return Object.keys(active)
      .map(function (key) {
        return active[key];
      })
      .sort(function (a, b) {
        if (a.path < b.path) return -1;
        if (a.path > b.path) return 1;
        return a.id - b.id;
      });
  }

  function segments(path) {
    var parts = path.split("/");
    var out = [];
    for (var i = 0; i < parts.length; i++) {
      if (parts[i].length > 0) out.push(parts[i]);
    }
    return out;
  }

  // Build a hierarchical tree where intermediate nodes are path segments and
  // leaves are concrete tasks. A leaf may share a segment name with a branch
  // (e.g. "a/b" and "a/b/c"); both are rendered.
  function buildTree() {
    var tasks = sortTasks();
    if (tasks.length === 0) {
      treeEmpty.style.display = "block";
      tree.innerHTML = "";
      return;
    }
    treeEmpty.style.display = "none";

    // root: { children: {seg: node}, leaves: [task] }
    var root = { children: Object.create(null), leaves: [] };
    for (var t = 0; t < tasks.length; t++) {
      var task = tasks[t];
      var segs = segments(task.path);
      var node = root;
      for (var s = 0; s < segs.length - 1; s++) {
        var seg = segs[s];
        var child = node.children[seg];
        if (!child) {
          child = { children: Object.create(null), leaves: [] };
          node.children[seg] = child;
        }
        node = child;
      }
      node.leaves.push(task);
    }

    tree.innerHTML = "";
    renderLevel(root, tree, "");
  }

  function renderLevel(node, parentEl, prefix) {
    var childKeys = Object.keys(node.children).sort();
    for (var c = 0; c < childKeys.length; c++) {
      var key = childKeys[c];
      var child = node.children[key];
      var childPrefix = prefix ? prefix + "/" + key : key;
      var branch = branchTemplate.content.firstElementChild.cloneNode(true);
      var toggle = branch.querySelector(".toggle");
      var label = branch.querySelector(".label");
      var count = branch.querySelector(".count");
      var ul = branch.querySelector("ul");
      label.textContent = key;
      var descendants = countDescendants(child);
      count.textContent = "(" + descendants + ")";
      toggle.addEventListener("click", makeToggle(branch, toggle));
      if (node.leaves.length > 0) {
        // leaves at this level render before children
      }
      renderLevel(child, ul, childPrefix);
      parentEl.appendChild(branch);
    }
    // Leaves for this exact segment path.
    node.leaves.sort(function (a, b) {
      if (a.path < b.path) return -1;
      if (a.path > b.path) return 1;
      return a.id - b.id;
    });
    for (var l = 0; l < node.leaves.length; l++) {
      parentEl.appendChild(renderLeaf(node.leaves[l]));
    }
  }

  function countDescendants(node) {
    var total = node.leaves.length;
    var keys = Object.keys(node.children);
    for (var i = 0; i < keys.length; i++) {
      total += countDescendants(node.children[keys[i]]);
    }
    return total;
  }

  function renderLeaf(task) {
    var li = leafTemplate.content.firstElementChild.cloneNode(true);
    var pathEl = li.querySelector(".path");
    var metaEl = li.querySelector(".meta");
    pathEl.textContent = task.path;
    if (task.detached) {
      li.className = "leaf detached";
      // meta content set via ::before; keep element empty otherwise.
    }
    metaEl.title = "id #" + task.id + (task.detached ? " (detached)" : "");
    return li;
  }

  function makeToggle(branch, toggle) {
    return function () {
      var collapsed = branch.classList.toggle("collapsed");
      toggle.setAttribute("aria-expanded", collapsed ? "false" : "true");
    };
  }

  function updateCounts() {
    var keys = Object.keys(active);
    var detached = 0;
    var roots = Object.create(null);
    for (var i = 0; i < keys.length; i++) {
      var task = active[keys[i]];
      if (task.detached) detached++;
      var top = segments(task.path)[0];
      if (top) roots[top] = true;
    }
    activeCount.textContent = String(keys.length);
    rootCount.textContent = String(Object.keys(roots).length);
    detachedCount.textContent = String(detached);
  }

  function applyReset(tasks) {
    active = Object.create(null);
    for (var i = 0; i < tasks.length; i++) {
      var t = tasks[i];
      active[String(t.id)] = t;
    }
    buildTree();
    updateCounts();
  }

  function applyStarted(task) {
    active[String(task.id)] = task;
    buildTree();
    updateCounts();
  }

  function applyEnded(ended) {
    delete active[String(ended.id)];
    buildTree();
    updateCounts();
    addLogRow(ended);
  }

  function addLogRow(ended) {
    var li = logRowTemplate.content.firstElementChild.cloneNode(true);
    li.querySelector(".outcome").textContent = ended.outcome;
    li.querySelector(".outcome").className = "outcome " + ended.outcome;
    li.querySelector(".path").textContent = ended.path;
    li.querySelector(".id").textContent = "#" + ended.id;
    logEl.insertBefore(li, logEl.firstChild);
    logRows.unshift(li);
    while (logRows.length > MAX_LOG_ENTRIES) {
      var old = logRows.pop();
      if (old.parentNode) old.parentNode.removeChild(old);
    }
  }

  function clearLog() {
    logEl.innerHTML = "";
    logRows = [];
  }

  function connect() {
    setConnectionState("connecting", "connecting");
    var es = new EventSource("/api/tasks/events");

    es.addEventListener("reset", function (ev) {
      setConnectionState("connected", "connected");
      try {
        var data = JSON.parse(ev.data);
        applyReset(data.tasks || []);
      } catch (err) {
        // Ignore malformed payload; the next reset will recover.
      }
    });

    es.addEventListener("started", function (ev) {
      try {
        applyStarted(JSON.parse(ev.data));
      } catch (err) {
        // ignore
      }
    });

    es.addEventListener("ended", function (ev) {
      try {
        applyEnded(JSON.parse(ev.data));
      } catch (err) {
        // ignore
      }
    });

    es.onopen = function () {
      // Connection state is confirmed on the first reset, but mark optimistic.
    };

    es.onerror = function () {
      setConnectionState("reconnecting", "disconnected");
      // EventSource reconnects automatically; the next reset is authoritative.
    };
  }

  var allCollapsed = false;
  expandBtn.addEventListener("click", function () {
    allCollapsed = !allCollapsed;
    var branches = tree.querySelectorAll(".branch");
    for (var i = 0; i < branches.length; i++) {
      var b = branches[i];
      if (allCollapsed) {
        b.classList.add("collapsed");
      } else {
        b.classList.remove("collapsed");
      }
      var t = b.querySelector(".toggle");
      if (t) t.setAttribute("aria-expanded", allCollapsed ? "false" : "true");
    }
    expandBtn.textContent = allCollapsed ? "Expand all" : "Collapse all";
    expandBtn.setAttribute("aria-expanded", allCollapsed ? "false" : "true");
  });

  clearBtn.addEventListener("click", clearLog);

  connect();
})();
