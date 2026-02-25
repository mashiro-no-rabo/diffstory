// TOC highlight via IntersectionObserver
(function() {
  var chapters = document.querySelectorAll('.chapter-header');
  var tocLinks = document.querySelectorAll('.toc a[data-chapter]');

  if (chapters.length === 0) return;

  var observer = new IntersectionObserver(function(entries) {
    entries.forEach(function(entry) {
      if (entry.isIntersecting) {
        tocLinks.forEach(function(link) { link.classList.remove('active'); });
        var id = entry.target.id;
        var activeLink = document.querySelector('.toc a[href="#' + id + '"]');
        if (activeLink) activeLink.classList.add('active');
      }
    });
  }, { rootMargin: '-10% 0px -80% 0px' });

  chapters.forEach(function(ch) { observer.observe(ch); });
})();

// Collapse/expand
document.querySelectorAll('.collapsible-header').forEach(function(header) {
  header.addEventListener('click', function() {
    header.parentElement.classList.toggle('open');
  });
});

// Keyboard navigation
(function() {
  var sections = Array.from(document.querySelectorAll('.chapter-header'));
  var currentIdx = -1;

  document.addEventListener('keydown', function(e) {
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

// Split view toggle + generation
(function() {
  var btn = document.getElementById('split-toggle');
  if (!btn) return;

  var saved = localStorage.getItem('diffstory-split');
  if (saved === 'true') {
    document.documentElement.classList.add('split-view');
  }

  btn.addEventListener('click', function() {
    var html = document.documentElement;
    html.classList.toggle('split-view');
    localStorage.setItem('diffstory-split', html.classList.contains('split-view') ? 'true' : 'false');
    generateSplitTables();
  });

  // Generate split tables on load if split view is active
  if (document.documentElement.classList.contains('split-view')) {
    generateSplitTables();
  }

  function generateSplitTables() {
    // Only generate once
    if (document.querySelector('.diff-split')) return;

    document.querySelectorAll('.diff-table').forEach(function(table) {
      var diffFile = table.closest('.diff-file');
      var isNewFile = diffFile && diffFile.querySelector('.badge-new') !== null;
      var isDeletedFile = diffFile && diffFile.querySelector('.badge-deleted') !== null;

      var split = document.createElement('table');
      split.className = 'diff-split';
      split.innerHTML = '<colgroup>' +
        '<col class="split-marker"><col class="split-code">' +
        '<col class="split-divider">' +
        '<col class="split-marker"><col class="split-code">' +
        '</colgroup>';

      var rows = table.querySelectorAll('tr');
      var lines = [];
      rows.forEach(function(row) {
        if (row.classList.contains('diff-hunk-header')) {
          lines.push({ type: 'header', text: row.querySelector('td').textContent });
        } else if (row.classList.contains('diff-line-add')) {
          lines.push({ type: 'add', text: row.querySelector('.diff-code').textContent });
        } else if (row.classList.contains('diff-line-del')) {
          lines.push({ type: 'del', text: row.querySelector('.diff-code').textContent });
        } else if (row.classList.contains('diff-line-ctx')) {
          lines.push({ type: 'ctx', text: row.querySelector('.diff-code').textContent });
        } else if (row.classList.contains('diff-line-noeof')) {
          lines.push({ type: 'noeof', text: row.querySelector('.diff-code').textContent });
        }
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
          paired.push({ type: line.type, text: line.text });
          i++;
        } else if (line.type === 'del') {
          var dels = [];
          while (i < lines.length && lines[i].type === 'del') {
            dels.push(lines[i].text);
            i++;
          }
          var adds = [];
          while (i < lines.length && lines[i].type === 'add') {
            adds.push(lines[i].text);
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
          paired.push({ type: 'pair', left: null, right: line.text });
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
          var lCode = p.left !== null ? esc(p.left) : '';
          var rCode = p.right !== null ? esc(p.right) : '';
          tr.innerHTML =
            '<td class="diff-marker ' + lCls + '">' + lMark + '</td>' +
            '<td class="diff-code ' + lCls + '">' + lCode + '</td>' +
            '<td class="split-divider"></td>' +
            '<td class="diff-marker ' + rCls + '">' + rMark + '</td>' +
            '<td class="diff-code ' + rCls + '">' + rCode + '</td>';
        }
        split.appendChild(tr);
      });

      table.parentNode.insertBefore(split, table.nextSibling);
    });
  }

  function esc(s) {
    var d = document.createElement('div');
    d.textContent = s;
    return d.innerHTML;
  }
})();
