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
