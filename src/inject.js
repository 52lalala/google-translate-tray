(function() {
  function focusInput() {
    var ta = document.querySelector('textarea');
    if (ta) {
      ta.focus();
      return true;
    }
    return false;
  }

  var pollTimer = setInterval(function() {
    if (focusInput()) {
      clearInterval(pollTimer);
    }
  }, 300);

  setTimeout(function() {
    clearInterval(pollTimer);
  }, 30000);

  document.addEventListener('visibilitychange', function() {
    if (document.visibilityState === 'visible') {
      focusInput();
    }
  });
})();
