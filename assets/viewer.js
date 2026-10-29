// TOC highlight via scroll position
(function() {
  var sections = Array.from(document.querySelectorAll('.story-section-header'));
  var tocLinks = document.querySelectorAll('.toc a[data-section]');

  if (sections.length === 0) return;

  function updateToc() {
    var scrollTop = window.scrollY || document.documentElement.scrollTop;
    var threshold = window.innerHeight * 0.15;
    var activeId = null;

    // Find the last section header that has scrolled past the threshold
    for (var i = sections.length - 1; i >= 0; i--) {
      if (sections[i].getBoundingClientRect().top <= threshold) {
        activeId = sections[i].id;
        break;
      }
    }

    tocLinks.forEach(function(link) {
      if (activeId && link.getAttribute('href') === '#' + activeId) {
        link.classList.add('active');
      } else {
        link.classList.remove('active');
      }
    });
  }

  var ticking = false;
  window.addEventListener('scroll', function() {
    if (!ticking) {
      requestAnimationFrame(function() { updateToc(); ticking = false; });
      ticking = true;
    }
  });
  updateToc();
})();

// Collapse/expand
document.querySelectorAll('.collapsible-header').forEach(function(header) {
  header.addEventListener('click', function() {
    header.parentElement.classList.toggle('open');
  });
});

// Fold/unfold individual diff-file blocks
document.querySelectorAll('.diff-fold-btn').forEach(function(btn) {
  btn.addEventListener('click', function(e) {
    e.stopPropagation();
    var diffFile = btn.closest('.diff-file');
    if (diffFile) diffFile.classList.toggle('folded');
  });
});

// "Viewed" checkbox — persisted per file, folds the diff while viewed,
// synced across blocks of the same file
(function() {
  var PREFIX = 'diffstory-viewed-';

  document.querySelectorAll('.diff-file-header[data-file]').forEach(function(header) {
    var key = header.getAttribute('data-file');
    var checkbox = header.querySelector('.viewed-checkbox');
    var file = header.closest('.diff-file');
    if (!key || !checkbox || !file) return;

    var stored = localStorage.getItem(PREFIX + key) === 'true';
    checkbox.checked = stored;
    if (stored) {
      file.classList.add('viewed', 'folded');
    }

    checkbox.addEventListener('change', function() {
      file.classList.toggle('viewed', checkbox.checked);
      file.classList.toggle('folded', checkbox.checked);
      localStorage.setItem(PREFIX + key, checkbox.checked ? 'true' : 'false');

      // Keep other blocks of the same file in sync
      document.querySelectorAll('.diff-file-header[data-file="' + CSS.escape(key) + '"]').forEach(function(other) {
        if (other === header) return;
        var otherBox = other.querySelector('.viewed-checkbox');
        if (otherBox) otherBox.checked = checkbox.checked;
        var otherFile = other.closest('.diff-file');
        if (otherFile) {
          otherFile.classList.toggle('viewed', checkbox.checked);
          otherFile.classList.toggle('folded', checkbox.checked);
        }
      });
    });
  });
})();

// Keyboard navigation
(function() {
  var sections = Array.from(document.querySelectorAll('.story-section-header'));
  var currentIdx = -1;

  document.addEventListener('keydown', function(e) {
    if (e.defaultPrevented) return;
    if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;

    if (e.key === 'j' || e.key === 'k') {
      e.preventDefault();
      if (e.key === 'j') {
        currentIdx = Math.min(currentIdx + 1, sections.length - 1);
      } else {
        currentIdx = Math.max(currentIdx - 1, 0);
      }
      if (sections[currentIdx]) {
        sections[currentIdx].scrollIntoView({ behavior: 'smooth', block: 'start' });
      }
    }
  });
})();

// Dark theme toggle
(function() {
  var btn = document.getElementById('theme-toggle');
  if (!btn) return;

  var saved = localStorage.getItem('diffstory-theme');
  if (saved === 'dark' || (!saved && window.matchMedia('(prefers-color-scheme: dark)').matches)) {
    document.documentElement.classList.add('dark');
  }

  btn.addEventListener('click', function() {
    var html = document.documentElement;
    html.classList.toggle('dark');
    localStorage.setItem('diffstory-theme', html.classList.contains('dark') ? 'dark' : 'light');
  });
})();

// Inline/split layout switch + split table generation
(function() {
  var inlineBtn = document.getElementById('inline-mode');
  var splitBtn = document.getElementById('split-mode');
  if (!inlineBtn || !splitBtn) return;

  function setSplit(split) {
    document.documentElement.classList.toggle('split-view', split);
    inlineBtn.setAttribute('aria-pressed', String(!split));
    splitBtn.setAttribute('aria-pressed', String(split));
    localStorage.setItem('diffstory-split', split ? 'true' : 'false');
    if (split) generateSplitTables();
  }

  inlineBtn.addEventListener('click', function() { setSplit(false); });
  splitBtn.addEventListener('click', function() { setSplit(true); });
  setSplit(localStorage.getItem('diffstory-split') === 'true');

  function generateSplitTables() {
    // Only generate once
    if (document.querySelector('.diff-split')) return;

    document.querySelectorAll('.diff-table').forEach(function(table) {
      var diffFile = table.closest('.diff-file');
      var isNewFile = diffFile && diffFile.querySelector('.badge-new') !== null;
      var isDeletedFile = diffFile && diffFile.querySelector('.badge-deleted') !== null;

      var split = document.createElement('table');
      split.className = 'diff-split';
      split.setAttribute('data-file', table.getAttribute('data-file') || '');
      split.innerHTML = '<colgroup>' +
        '<col class="split-marker"><col class="split-code">' +
        '<col class="split-divider">' +
        '<col class="split-marker"><col class="split-code">' +
        '</colgroup>';

      var rows = table.querySelectorAll('tr');
      var lines = [];
      rows.forEach(function(row) {
        var type = null;
        if (row.classList.contains('diff-hunk-header')) {
          lines.push({ type: 'header', text: row.querySelector('td').textContent });
          return;
        } else if (row.classList.contains('diff-line-add')) {
          type = 'add';
        } else if (row.classList.contains('diff-line-del')) {
          type = 'del';
        } else if (row.classList.contains('diff-line-ctx')) {
          type = 'ctx';
        } else if (row.classList.contains('diff-line-noeof')) {
          type = 'noeof';
        }
        if (!type) return;
        lines.push({
          type: type,
          text: row.querySelector('.diff-code').textContent,
          old: row.getAttribute('data-old-line'),
          nw: row.getAttribute('data-new-line')
        });
      });

      // For new files: show everything on the right only
      if (isNewFile) {
        lines.forEach(function(line) {
          var tr = document.createElement('tr');
          if (line.type === 'header') {
            tr.className = 'diff-hunk-header';
            tr.innerHTML = '<td colspan="5">' + esc(line.text) + '</td>';
          } else {
            tr.innerHTML =
              '<td class="diff-marker split-empty"></td>' +
              '<td class="diff-code split-empty"></td>' +
              '<td class="split-divider"></td>' +
              '<td class="diff-marker split-add">+</td>' +
              '<td class="diff-code split-add">' + esc(line.text) + '</td>';
            tagLines(tr, null, line.nw);
          }
          split.appendChild(tr);
        });
        table.parentNode.insertBefore(split, table.nextSibling);
        return;
      }

      // For deleted files: show everything on the left only
      if (isDeletedFile) {
        lines.forEach(function(line) {
          var tr = document.createElement('tr');
          if (line.type === 'header') {
            tr.className = 'diff-hunk-header';
            tr.innerHTML = '<td colspan="5">' + esc(line.text) + '</td>';
          } else {
            tr.innerHTML =
              '<td class="diff-marker split-del">-</td>' +
              '<td class="diff-code split-del">' + esc(line.text) + '</td>' +
              '<td class="split-divider"></td>' +
              '<td class="diff-marker split-empty"></td>' +
              '<td class="diff-code split-empty"></td>';
            tagLines(tr, line.old, null);
          }
          split.appendChild(tr);
        });
        table.parentNode.insertBefore(split, table.nextSibling);
        return;
      }

      // Normal files: pair deletions with additions side-by-side
      var paired = [];
      var i = 0;
      while (i < lines.length) {
        var line = lines[i];
        if (line.type === 'header') {
          paired.push({ type: 'header', text: line.text });
          i++;
        } else if (line.type === 'ctx' || line.type === 'noeof') {
          paired.push({ type: line.type, text: line.text, old: line.old, nw: line.nw });
          i++;
        } else if (line.type === 'del') {
          var dels = [];
          while (i < lines.length && lines[i].type === 'del') {
            dels.push(lines[i]);
            i++;
          }
          var adds = [];
          while (i < lines.length && lines[i].type === 'add') {
            adds.push(lines[i]);
            i++;
          }
          var max = Math.max(dels.length, adds.length);
          for (var j = 0; j < max; j++) {
            paired.push({
              type: 'pair',
              left: j < dels.length ? dels[j] : null,
              right: j < adds.length ? adds[j] : null
            });
          }
        } else if (line.type === 'add') {
          paired.push({ type: 'pair', left: null, right: line });
          i++;
        }
      }

      paired.forEach(function(p) {
        var tr = document.createElement('tr');
        if (p.type === 'header') {
          tr.className = 'diff-hunk-header';
          tr.innerHTML = '<td colspan="5">' + esc(p.text) + '</td>';
        } else if (p.type === 'ctx') {
          tr.innerHTML =
            '<td class="diff-marker split-ctx"> </td>' +
            '<td class="diff-code split-ctx">' + esc(p.text) + '</td>' +
            '<td class="split-divider"></td>' +
            '<td class="diff-marker split-ctx"> </td>' +
            '<td class="diff-code split-ctx">' + esc(p.text) + '</td>';
          tagLines(tr, p.old, p.nw);
        } else if (p.type === 'noeof') {
          tr.innerHTML =
            '<td class="diff-marker split-empty"></td>' +
            '<td class="diff-code split-empty"></td>' +
            '<td class="split-divider"></td>' +
            '<td class="diff-marker split-empty"></td>' +
            '<td class="diff-code split-empty" style="color:var(--fg-muted);font-style:italic">' + esc(p.text) + '</td>';
        } else if (p.type === 'pair') {
          var lCls = p.left !== null ? 'split-del' : 'split-empty';
          var rCls = p.right !== null ? 'split-add' : 'split-empty';
          var lMark = p.left !== null ? '-' : '';
          var rMark = p.right !== null ? '+' : '';
          var lCode = p.left !== null ? esc(p.left.text) : '';
          var rCode = p.right !== null ? esc(p.right.text) : '';
          tr.innerHTML =
            '<td class="diff-marker ' + lCls + '">' + lMark + '</td>' +
            '<td class="diff-code ' + lCls + '">' + lCode + '</td>' +
            '<td class="split-divider"></td>' +
            '<td class="diff-marker ' + rCls + '">' + rMark + '</td>' +
            '<td class="diff-code ' + rCls + '">' + rCode + '</td>';
          tagLines(tr, p.left && p.left.old, p.right && p.right.nw);
        }
        split.appendChild(tr);
      });

      table.parentNode.insertBefore(split, table.nextSibling);
    });
  }

  function tagLines(tr, oldLn, newLn) {
    var cells = tr.querySelectorAll('td.diff-code');
    if (oldLn) cells[0].setAttribute('data-old-line', oldLn);
    if (newLn) cells[1].setAttribute('data-new-line', newLn);
  }

  function esc(s) {
    var d = document.createElement('div');
    d.textContent = s;
    return d.innerHTML;
  }
})();

// Review comments: select text in the story and start typing to comment,
// then copy all comments as a prompt for an agent session
(function() {
  var root = document.documentElement;
  var content = document.querySelector('.content');
  var drawerEl = document.getElementById('comments-drawer');
  if (!content || !drawerEl) return;

  function $(id) { return document.getElementById(id); }

  var MAX_SNIPPET_LINES = 20;
  var STORAGE_PREFIX = 'diffstory-notes-';
  var STORAGE_KEY = STORAGE_PREFIX + (drawerEl.getAttribute('data-render-id') || document.title);
  var prUrl = drawerEl.getAttribute('data-pr-url') || '';

  var sections = [];
  var intro = document.querySelector('.story-intro');
  if (intro) sections.push({ el: intro, id: intro.id, title: 'Overview' });
  document.querySelectorAll('.story-section').forEach(function(el) {
    var header = el.querySelector('.story-section-header');
    sections.push({ el: el, id: header.id, title: header.querySelector('h3').textContent.trim() });
  });
  var uncategorized = $('uncategorized');
  if (uncategorized) sections.push({ el: uncategorized, id: 'uncategorized', title: 'Uncategorized' });

  var selectedId = null;
  var pending = null;
  var editingId = null;

  pruneOldRenders();
  var notes = load().filter(function(n) { return n && info(n) && typeof n.body === 'string'; });

  function pruneOldRenders() {
    try {
      var stale = [];
      for (var i = 0; i < localStorage.length; i++) {
        var key = localStorage.key(i);
        if (key && key.indexOf(STORAGE_PREFIX) === 0 && key !== STORAGE_KEY) stale.push(key);
      }
      stale.forEach(function(key) { localStorage.removeItem(key); });
    } catch (e) {}
  }

  function load() {
    try {
      var stored = JSON.parse(localStorage.getItem(STORAGE_KEY));
      return Array.isArray(stored) ? stored : [];
    } catch (e) {
      return [];
    }
  }

  function save() {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(notes));
    } catch (e) {}
  }

  function info(n) {
    for (var i = 0; i < sections.length; i++) {
      if (sections[i].id === n.section) return sections[i];
    }
    return null;
  }

  function sectionFor(el) {
    var host = el.closest('.story-section, #uncategorized, .story-intro');
    if (!host) {
      // Group headers belong to the first section of their group
      var group = el.closest('.story-group');
      host = group && group.querySelector('.story-section');
    }
    for (var i = 0; host && i < sections.length; i++) {
      if (sections[i].el === host) return sections[i];
    }
    return null;
  }

  function toast(text) {
    var el = $('toast');
    el.textContent = text;
    el.classList.add('visible');
    clearTimeout(toast.timer);
    toast.timer = setTimeout(function() { el.classList.remove('visible'); }, 2200);
  }

  // --- Selection capture ---

  // Part of `range` that falls inside `node`
  function overlap(range, node) {
    var r = document.createRange();
    r.selectNodeContents(node);
    if (range.compareBoundaryPoints(Range.START_TO_START, r) > 0) r.setStart(range.startContainer, range.startOffset);
    if (range.compareBoundaryPoints(Range.END_TO_END, r) < 0) r.setEnd(range.endContainer, range.endOffset);
    return r;
  }

  function containsNode(range, node) {
    var r = document.createRange();
    r.selectNodeContents(node);
    return range.compareBoundaryPoints(Range.START_TO_START, r) <= 0 &&
      range.compareBoundaryPoints(Range.END_TO_END, r) >= 0;
  }

  function isVisible(el) {
    return el.offsetParent !== null;
  }

  function closestCell(node) {
    var el = node.nodeType === 1 ? node : node.parentElement;
    return el ? el.closest('td') : null;
  }

  function cellLine(cell) {
    var split = cell.closest('table').classList.contains('diff-split');
    var src = split ? cell : cell.closest('tr');
    var marker;
    if (split) {
      marker = cell.classList.contains('split-add') ? '+' : cell.classList.contains('split-del') ? '-' : ' ';
    } else {
      var row = cell.closest('tr');
      marker = row.classList.contains('diff-line-add') ? '+' : row.classList.contains('diff-line-del') ? '-' : ' ';
    }
    return {
      marker: marker,
      text: cell.textContent,
      old: src.getAttribute('data-old-line'),
      nw: src.getAttribute('data-new-line')
    };
  }

  function rangeLabel(nums) {
    var min = Math.min.apply(null, nums);
    var max = Math.max.apply(null, nums);
    return min === max ? 'L' + min : 'L' + min + '-L' + max;
  }

  function captureCode(range) {
    var tables = Array.from(content.querySelectorAll('table.diff-table, table.diff-split')).filter(function(t) {
      return isVisible(t) && range.intersectsNode(t);
    });
    var chunks = [];

    tables.forEach(function(table) {
      var cells = Array.from(table.querySelectorAll('td.diff-code')).filter(function(cell) {
        if (cell.classList.contains('split-empty')) return false;
        if (!range.intersectsNode(cell)) return false;
        if (!overlap(range, cell).collapsed) return true;
        return cell.textContent === '' && containsNode(range, cell);
      });

      // In split view, a selection within one column should not pick up the other
      if (table.classList.contains('diff-split')) {
        var startCell = closestCell(range.startContainer);
        var endCell = closestCell(range.endContainer);
        if (startCell && endCell && startCell.cellIndex === endCell.cellIndex) {
          cells = cells.filter(function(c) { return c.cellIndex === startCell.cellIndex; });
        }
      }
      if (cells.length === 0) return;

      var lines = cells.map(cellLine);
      var newNums = lines.filter(function(l) { return l.nw; }).map(function(l) { return +l.nw; });
      var oldNums = lines.filter(function(l) { return l.old; }).map(function(l) { return +l.old; });
      var file = table.getAttribute('data-file');

      var selected = null;
      if (cells.length === 1) {
        var part = overlap(range, cells[0]).toString();
        if (part.trim() && part !== cells[0].textContent) selected = part;
      }

      chunks.push({
        where: newNums.length ? file + ':' + rangeLabel(newNums) : file + ' (old):' + rangeLabel(oldNums),
        anchor: newNums.length
          ? { file: file, line: newNums[0], side: 'new' }
          : { file: file, line: oldNums[0], side: 'old' },
        lines: lines,
        selected: selected
      });
    });

    if (chunks.length === 0) return null;

    var snippet = [];
    chunks.forEach(function(c) {
      c.lines.forEach(function(l) { snippet.push(l.marker + l.text); });
    });
    if (snippet.length > MAX_SNIPPET_LINES) {
      snippet = snippet.slice(0, MAX_SNIPPET_LINES).concat(['... (' + (snippet.length - MAX_SNIPPET_LINES) + ' more lines)']);
    }

    return {
      kind: 'code',
      location: chunks.map(function(c) { return c.where; }).join(', '),
      anchor: chunks[0].anchor,
      quote: snippet.join('\n'),
      selected: chunks.length === 1 ? chunks[0].selected : null
    };
  }

  // Generated ids wouldn't survive a reload, so anchor to the nearest rendered id
  function stableAnchor(el) {
    var withId = el && el.closest('[id]');
    return withId && content.contains(withId) ? { id: withId.id } : null;
  }

  function captureText(range, el) {
    var text = range.toString().trim();
    if (!text) return null;
    var location = 'Story';
    var block;

    if ((block = el.closest('.story-description'))) {
      location = 'Story description';
    } else if ((block = el.closest('.story-group-header'))) {
      location = 'Group "' + block.querySelector('.story-group-title').textContent.trim() + '" description';
    } else if ((block = el.closest('.story-section-header'))) {
      location = 'Section "' + block.querySelector('h3').textContent.trim() + '" description';
    } else if ((block = el.closest('.hunk-note'))) {
      location = 'Note on ' + block.closest('.diff-file').querySelector('.diff-file-header').getAttribute('data-file');
    } else if ((block = el.closest('.page-header'))) {
      location = 'PR title';
    } else if ((block = el.closest('.diff-file-header'))) {
      location = 'File ' + block.getAttribute('data-file');
    }

    return {
      kind: 'text',
      location: location,
      anchor: stableAnchor(block || el),
      quote: text.slice(0, 3000)
    };
  }

  function capture() {
    var sel = window.getSelection();
    if (!sel || sel.isCollapsed || sel.rangeCount === 0) return null;
    var range = sel.getRangeAt(0);
    if (!content.contains(range.commonAncestorContainer)) return null;
    var node = range.startContainer;
    var el = node.nodeType === 1 ? node : node.parentElement;
    if (!el || el.closest('button')) return null;

    var section = sectionFor(el);
    if (!section) return null;
    var note = captureCode(range) || captureText(range, el);
    if (!note) return null;
    note.section = section.id;
    return note;
  }

  // --- Rendering ---

  function getTarget(n) {
    var sec = info(n).el;
    var a = n.anchor;
    if (a && a.id) return document.getElementById(a.id) || sec;
    if (a && a.file) {
      var tables = Array.from(sec.querySelectorAll('.diff-table, .diff-split')).filter(function(t) {
        return isVisible(t) && t.getAttribute('data-file') === a.file;
      });
      var attr = a.side === 'old' ? 'data-old-line' : 'data-new-line';
      for (var i = 0; i < tables.length; i++) {
        var hit = a.line && tables[i].querySelector('[' + attr + '="' + a.line + '"]');
        if (hit) return hit;
      }
      if (tables[0]) return tables[0];
    }
    return sec;
  }

  function reveal(n) {
    var sec = info(n).el;
    if (sec.classList.contains('collapsible')) sec.classList.add('open');
    if (n.anchor && n.anchor.file) {
      sec.querySelectorAll('.diff-file').forEach(function(f) {
        var header = f.querySelector('.diff-file-header');
        if (header && header.getAttribute('data-file') === n.anchor.file) f.classList.remove('folded');
      });
    }
  }

  function jump(n) {
    selectedId = n.id;
    reveal(n);
    render();
    document.querySelectorAll('.source-active').forEach(function(e) { e.classList.remove('source-active'); });
    var target = getTarget(n);
    target.classList.add('source-active');
    target.scrollIntoView({ behavior: 'smooth', block: 'center' });
  }

  function makeButton(label, cls, fn) {
    var b = document.createElement('button');
    b.type = 'button';
    b.textContent = label;
    if (cls) b.className = cls;
    b.addEventListener('click', fn);
    return b;
  }

  function card(n) {
    var el = document.createElement('article');
    el.className = 'list-note' + (selectedId === n.id ? ' selected' : '');
    el.setAttribute('data-note-id', n.id);
    el.appendChild(makeButton(n.location, 'comment-location', function() { jump(n); }));

    if (n.quote) {
      var details = document.createElement('details');
      var summary = document.createElement('summary');
      var pre = document.createElement('pre');
      summary.textContent = 'Quoted context';
      pre.textContent = n.quote;
      details.appendChild(summary);
      details.appendChild(pre);
      el.appendChild(details);
    }

    var text = document.createElement('p');
    text.className = 'comment-text';
    text.textContent = n.body;
    el.appendChild(text);

    el.appendChild(makeButton('Jump to section →', 'jump-link', function() { jump(n); }));

    var tools = document.createElement('div');
    tools.className = 'comment-tools';
    tools.appendChild(makeButton('Edit', '', function() { edit(n); }));
    tools.appendChild(makeButton('Delete', '', function() { remove(n); }));
    el.appendChild(tools);
    return el;
  }

  function notesIn(s) {
    return notes.filter(function(n) { return n.section === s.id; });
  }

  function render() {
    $('comment-count').textContent = notes.length;
    $('drawer-count').textContent = notes.length;

    var list = $('all-comments');
    list.replaceChildren();
    sections.forEach(function(s) {
      var ns = notesIn(s);
      if (!ns.length) return;
      var h = document.createElement('h3');
      h.className = 'comment-group-title';
      h.textContent = s.title + ' · ' + ns.length;
      list.appendChild(h);
      ns.forEach(function(n) { list.appendChild(card(n)); });
    });
    if (!notes.length) {
      var p = document.createElement('p');
      p.className = 'empty-comments';
      p.textContent = 'No comments yet. Select story text or code, then start typing.';
      list.appendChild(p);
    }
  }

  // --- Editing ---

  function openEditor(n, value, isEdit) {
    drawer(true);
    $('editor-heading').textContent = isEdit ? 'Edit comment' : 'New comment';
    $('editor-location').textContent = n.location;
    $('editor-quote').textContent = n.quote || '';
    $('editor-quote').hidden = !n.quote;
    $('comment-body').value = value;
    $('comment-editor').hidden = false;
    var ta = $('comment-body');
    ta.focus();
    ta.setSelectionRange(ta.value.length, ta.value.length);
  }

  function closeEditor() {
    $('comment-editor').hidden = true;
    editingId = null;
    pending = null;
  }

  function edit(n) {
    editingId = n.id;
    pending = n;
    openEditor(n, n.body, true);
  }

  function remove(n) {
    notes = notes.filter(function(x) { return x.id !== n.id; });
    save();
    render();
    toast('Comment deleted');
  }

  function saveEditor() {
    var body = $('comment-body').value.trim();
    if (!body) return false;
    if (editingId) {
      var existing = notes.find(function(n) { return n.id === editingId; });
      if (existing) existing.body = body;
    } else {
      var n = Object.assign({}, pending, { id: 'note-' + Date.now() + '-' + Math.random().toString(36).slice(2, 8), body: body });
      notes.push(n);
      selectedId = n.id;
    }
    save();
    closeEditor();
    render();
    var sel = window.getSelection();
    if (sel) sel.removeAllRanges();
    toast('Comment saved');
    return true;
  }

  $('comment-editor').addEventListener('submit', function(e) {
    e.preventDefault();
    saveEditor();
  });

  $('cancel-comment').addEventListener('click', closeEditor);

  $('comment-body').addEventListener('keydown', function(e) {
    if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      saveEditor();
    }
  });

  // Capture phase so the j/k section navigation doesn't swallow the first keystroke
  document.addEventListener('keydown', function(e) {
    if (e.key === 'Escape') {
      if (!$('comment-editor').hidden) closeEditor();
      else drawer(false);
      return;
    }
    if (e.ctrlKey || e.metaKey || e.altKey || e.isComposing || e.key.length !== 1) return;
    if (e.target.closest && e.target.closest('input, textarea, select, button, [contenteditable]')) return;
    var n = capture();
    if (!n) return;
    e.preventDefault();
    pending = n;
    editingId = null;
    openEditor(n, e.key, false);
  }, true);

  // --- Drawer, outline, layout switch ---

  function drawer(open) {
    root.classList.toggle('drawer-open', open);
    drawerEl.hidden = !open;
    $('comments-toggle').setAttribute('aria-expanded', String(open));
  }

  $('comments-toggle').addEventListener('click', function() { drawer(!root.classList.contains('drawer-open')); });
  $('close-comments').addEventListener('click', function() { drawer(false); });

  $('outline-toggle').addEventListener('click', function() {
    var open = root.classList.toggle('outline-open');
    $('outline-toggle').setAttribute('aria-expanded', String(open));
  });
  $('toc').addEventListener('click', function(e) {
    if (!e.target.closest('a')) return;
    root.classList.remove('outline-open');
    $('outline-toggle').setAttribute('aria-expanded', 'false');
  });

  // --- Prompt export ---

  function buildPrompt() {
    var title = document.querySelector('.page-header h1');
    var head = 'Review comments on the PR "' + (title ? title.textContent.trim() : document.title) + '"' +
      (prUrl ? ' (' + prUrl + ')' : '') + '.\n' +
      'Address each comment below. Line numbers refer to the new version of the file unless marked (old).\n';

    var parts = notes.map(function(n, i) {
      var out = '## ' + (i + 1) + '. ' + n.location + '\n';
      var sec = info(n);
      if (sec) out += 'Story section: ' + sec.title + '\n';
      out += '\n';
      if (n.kind === 'code') {
        out += '```diff\n' + n.quote + '\n```\n';
        if (n.selected) out += 'Selected: `' + n.selected + '`\n';
        out += '\n';
      } else if (n.quote) {
        out += n.quote.split('\n').map(function(l) { return '> ' + l; }).join('\n') + '\n\n';
      }
      return out + n.body.trim() + '\n';
    });

    return head + '\n' + parts.join('\n');
  }

  function copyText(text) {
    if (navigator.clipboard && navigator.clipboard.writeText) {
      return navigator.clipboard.writeText(text).catch(function() { return fallbackCopy(text); });
    }
    return fallbackCopy(text);
  }

  function fallbackCopy(text) {
    var t = document.createElement('textarea');
    t.value = text;
    t.style.position = 'fixed';
    t.style.left = '-9999px';
    document.body.appendChild(t);
    t.select();
    var ok = document.execCommand('copy');
    t.remove();
    return ok ? Promise.resolve() : Promise.reject(new Error('copy failed'));
  }

  $('copy-comments').addEventListener('click', function() {
    if (!notes.length) {
      toast('No comments to copy');
      return;
    }
    copyText(buildPrompt()).then(function() {
      notes = [];
      save();
      render();
      toast('Prompt copied, comments cleared');
    }, function() {
      toast('Copy failed');
    });
  });

  $('clear-comments').addEventListener('click', function() {
    if (!notes.length || !confirm('Delete all ' + notes.length + ' comments?')) return;
    notes = [];
    save();
    render();
    toast('Comments cleared');
  });

  render();
})();
