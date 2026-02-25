// TOC highlight via scroll position
(function() {
  var chapters = Array.from(document.querySelectorAll('.chapter-header'));
  var tocLinks = document.querySelectorAll('.toc a[data-chapter]');

  if (chapters.length === 0) return;

  function updateToc() {
    var scrollTop = window.scrollY || document.documentElement.scrollTop;
    var threshold = window.innerHeight * 0.15;
    var activeId = null;

    // Find the last chapter header that has scrolled past the threshold
    for (var i = chapters.length - 1; i >= 0; i--) {
      if (chapters[i].getBoundingClientRect().top <= threshold) {
        activeId = chapters[i].id;
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
        } else if (row.classList.contains('comment-row')) {
          // Preserve comment rows - add as-is
          lines.push({ type: 'comment', element: row });
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
          if (line.type === 'comment') {
            var tr = document.createElement('tr');
            tr.className = 'comment-row';
            tr.innerHTML = '<td colspan="5">' + line.element.querySelector('td').innerHTML + '</td>';
            split.appendChild(tr);
            return;
          }
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
          if (line.type === 'comment') {
            var tr = document.createElement('tr');
            tr.className = 'comment-row';
            tr.innerHTML = '<td colspan="5">' + line.element.querySelector('td').innerHTML + '</td>';
            split.appendChild(tr);
            return;
          }
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
        } else if (line.type === 'comment') {
          paired.push({ type: 'comment', element: line.element });
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
        } else if (p.type === 'comment') {
          tr.className = 'comment-row';
          tr.innerHTML = '<td colspan="5">' + p.element.querySelector('td').innerHTML + '</td>';
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

// Comments visibility toggle
(function() {
  var btn = document.getElementById('comments-toggle');
  if (!btn) return;

  var saved = localStorage.getItem('diffstory-comments');
  if (saved === 'false') {
    document.documentElement.classList.remove('show-comments');
  }

  btn.addEventListener('click', function() {
    var html = document.documentElement;
    html.classList.toggle('show-comments');
    localStorage.setItem('diffstory-comments', html.classList.contains('show-comments') ? 'true' : 'false');
  });
})();

// Click-to-comment on diff lines + reply to threads
(function() {
  var prMeta = document.getElementById('pr-meta');
  if (!prMeta) return;

  var repo = prMeta.getAttribute('data-pr-repo');
  var number = prMeta.getAttribute('data-pr-number');
  var headSha = prMeta.getAttribute('data-pr-head-sha');

  if (!repo || !number || !headSha) return;

  var tpl = document.getElementById('comment-form-tpl');
  if (!tpl) return;

  // Click on diff line numbers to add a comment
  document.addEventListener('click', function(e) {
    var lineNumCell = e.target.closest('.diff-line-num');
    if (!lineNumCell) return;

    var row = lineNumCell.closest('tr');
    if (!row) return;

    var file = row.getAttribute('data-file');
    var line = row.getAttribute('data-line');
    if (!file || !line) return;

    // Check if a form already exists for this line
    var nextRow = row.nextElementSibling;
    if (nextRow && nextRow.classList.contains('comment-form-row')) {
      // Focus the existing textarea
      var ta = nextRow.querySelector('.comment-textarea');
      if (ta) ta.focus();
      return;
    }

    // Insert comment form
    var formRow = document.createElement('tr');
    formRow.className = 'comment-form-row';
    var td = document.createElement('td');
    td.setAttribute('colspan', '3');
    var form = tpl.content.cloneNode(true).querySelector('.comment-form');

    var draftKey = 'diffstory-draft-' + file + '-' + line;
    var textarea = form.querySelector('.comment-textarea');
    var savedDraft = localStorage.getItem(draftKey);
    if (savedDraft) textarea.value = savedDraft;

    // Auto-save draft
    textarea.addEventListener('input', function() {
      localStorage.setItem(draftKey, textarea.value);
    });

    // Save to batch — ensure draft is stored then collapse the form
    form.querySelector('.comment-btn-copy').addEventListener('click', function() {
      var text = textarea.value.trim();
      if (!text) return;
      localStorage.setItem(draftKey, text);
      row.classList.add('has-draft');
      formRow.classList.add('comment-form-saved');
      var btn = form.querySelector('.comment-btn-copy');
      btn.textContent = 'Saved!';
      btn.classList.add('comment-btn-copied');
      setTimeout(function() { formRow.remove(); }, 600);
    });

    // Cancel — discard draft and remove the form
    form.querySelector('.comment-btn-reset').addEventListener('click', function() {
      localStorage.removeItem(draftKey);
      row.classList.remove('has-draft');
      formRow.remove();
    });

    td.appendChild(form);
    formRow.appendChild(td);
    row.parentNode.insertBefore(formRow, row.nextSibling);
    textarea.focus();
  });

  // Reply to existing thread
  document.addEventListener('click', function(e) {
    var replyBtn = e.target.closest('.reply-btn');
    if (!replyBtn) return;

    e.preventDefault();
    var commentId = replyBtn.getAttribute('data-comment-id');
    if (!commentId) return;

    var thread = replyBtn.closest('.comment-thread');
    if (!thread) return;

    // Check if reply form already exists
    if (thread.querySelector('.comment-form')) {
      thread.querySelector('.comment-textarea').focus();
      return;
    }

    var form = tpl.content.cloneNode(true).querySelector('.comment-form');
    var draftKey = 'diffstory-reply-' + commentId;
    var textarea = form.querySelector('.comment-textarea');
    textarea.placeholder = 'Write a reply...';
    var savedDraft = localStorage.getItem(draftKey);
    if (savedDraft) textarea.value = savedDraft;

    textarea.addEventListener('input', function() {
      localStorage.setItem(draftKey, textarea.value);
    });

    var copyBtn = form.querySelector('.comment-btn-copy');
    copyBtn.addEventListener('click', function() {
      var text = textarea.value.trim();
      if (!text) return;
      localStorage.setItem(draftKey, text);
      copyBtn.textContent = 'Saved!';
      copyBtn.classList.add('comment-btn-copied');
      setTimeout(function() { form.remove(); }, 600);
    });

    form.querySelector('.comment-btn-reset').addEventListener('click', function() {
      localStorage.removeItem(draftKey);
      form.remove();
    });

    thread.appendChild(form);
    textarea.focus();
  });

  // Export all draft comments as a batch script
  var exportBtn = document.getElementById('export-comments');
  if (exportBtn) {
    exportBtn.addEventListener('click', function() {
      var commands = [];

      // Collect new line comments from open forms
      document.querySelectorAll('.comment-form-row').forEach(function(formRow) {
        var textarea = formRow.querySelector('.comment-textarea');
        if (!textarea || !textarea.value.trim()) return;
        var prevRow = formRow.previousElementSibling;
        if (!prevRow) return;
        var file = prevRow.getAttribute('data-file');
        var line = prevRow.getAttribute('data-line');
        if (!file || !line) return;
        commands.push(
          'gh api --method POST repos/' + repo + '/pulls/' + number + '/comments' +
          ' -f body=' + shellQuote(textarea.value.trim()) +
          ' -f path=' + shellQuote(file) +
          ' -F line=' + line +
          ' -f commit_id=' + headSha +
          ' -f side=RIGHT'
        );
      });

      // Collect reply drafts from open forms in threads
      document.querySelectorAll('.comment-thread .comment-form').forEach(function(form) {
        var textarea = form.querySelector('.comment-textarea');
        if (!textarea || !textarea.value.trim()) return;
        var replyBtn = form.parentElement.querySelector('.reply-btn');
        if (!replyBtn) return;
        var commentId = replyBtn.getAttribute('data-comment-id');
        if (!commentId) return;
        commands.push(
          'gh api --method POST repos/' + repo + '/pulls/' + number + '/comments' +
          ' -f body=' + shellQuote(textarea.value.trim()) +
          ' -F in_reply_to=' + commentId
        );
      });

      // Also check localStorage for drafts without open forms
      for (var i = 0; i < localStorage.length; i++) {
        var key = localStorage.key(i);
        if (!key) continue;
        if (key.startsWith('diffstory-draft-')) {
          var val = localStorage.getItem(key);
          if (!val || !val.trim()) continue;
          // Check if this draft already has an open form (already collected above)
          var parts = key.substring('diffstory-draft-'.length);
          var lastDash = parts.lastIndexOf('-');
          if (lastDash === -1) continue;
          var file = parts.substring(0, lastDash);
          var line = parts.substring(lastDash + 1);
          var existingForm = document.querySelector('tr[data-file="' + CSS.escape(file) + '"][data-line="' + line + '"] + .comment-form-row');
          if (existingForm) continue; // already collected
          commands.push(
            'gh api --method POST repos/' + repo + '/pulls/' + number + '/comments' +
            ' -f body=' + shellQuote(val.trim()) +
            ' -f path=' + shellQuote(file) +
            ' -F line=' + line +
            ' -f commit_id=' + headSha +
            ' -f side=RIGHT'
          );
        }
        if (key.startsWith('diffstory-reply-')) {
          var val = localStorage.getItem(key);
          if (!val || !val.trim()) continue;
          var commentId = key.substring('diffstory-reply-'.length);
          var existingForm = document.querySelector('.reply-btn[data-comment-id="' + commentId + '"]');
          if (existingForm && existingForm.closest('.comment-thread').querySelector('.comment-form')) continue;
          commands.push(
            'gh api --method POST repos/' + repo + '/pulls/' + number + '/comments' +
            ' -f body=' + shellQuote(val.trim()) +
            ' -F in_reply_to=' + commentId
          );
        }
      }

      if (commands.length === 0) {
        exportBtn.textContent = 'No drafts';
        setTimeout(function() { exportBtn.textContent = '\uD83D\uDCE6'; }, 1500);
        return;
      }

      var script = '#!/bin/sh\n# ' + commands.length + ' comment(s) for PR #' + number + '\nset -e\n\n' +
        commands.join('\n\n') + '\n\necho "Posted ' + commands.length + ' comment(s)"\n';

      navigator.clipboard.writeText(script).then(function() {
        // Clear all drafts
        var keysToRemove = [];
        for (var k = 0; k < localStorage.length; k++) {
          var lsKey = localStorage.key(k);
          if (lsKey && (lsKey.startsWith('diffstory-draft-') || lsKey.startsWith('diffstory-reply-'))) {
            keysToRemove.push(lsKey);
          }
        }
        keysToRemove.forEach(function(k) { localStorage.removeItem(k); });

        // Remove draft indicators and open forms
        document.querySelectorAll('.has-draft').forEach(function(el) { el.classList.remove('has-draft'); });
        document.querySelectorAll('.comment-form-row').forEach(function(el) { el.remove(); });
        document.querySelectorAll('.comment-thread .comment-form').forEach(function(el) { el.remove(); });

        exportBtn.textContent = commands.length + ' copied!';
        exportBtn.classList.add('comment-btn-copied');
        setTimeout(function() {
          exportBtn.textContent = '\uD83D\uDCE6';
          exportBtn.classList.remove('comment-btn-copied');
        }, 2000);
      });
    });
  }

  // Restore saved drafts on page load — mark rows with draft indicators
  (function restoreDrafts() {
    for (var i = 0; i < localStorage.length; i++) {
      var key = localStorage.key(i);
      if (!key) continue;

      if (key.startsWith('diffstory-draft-')) {
        var parts = key.substring('diffstory-draft-'.length);
        var lastDash = parts.lastIndexOf('-');
        if (lastDash === -1) continue;
        var file = parts.substring(0, lastDash);
        var line = parts.substring(lastDash + 1);

        // Mark the diff line row with a draft indicator
        var rows = document.querySelectorAll('tr[data-file="' + CSS.escape(file) + '"][data-line="' + line + '"]');
        if (rows.length > 0) {
          rows[0].classList.add('has-draft');
        }
      }
    }
  })();

  function shellQuote(s) {
    return "'" + s.replace(/'/g, "'\\''") + "'";
  }
})();
