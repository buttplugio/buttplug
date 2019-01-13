/* global MATOMO_SITE_ID, MATOMO_TRACKER_URL, MATOMO_ENABLE_LINK_TRACKING, _paq */

export default ({ router }) => {
  // Google analytics integration
  if (process.env.NODE_ENV === 'production' && typeof window !== 'undefined' && MATOMO_SITE_ID && MATOMO_TRACKER_URL) {
    // We're in SSR space here, meaning that we have to explictly attach _paq to
    // the window in order to store it globally.
    if (window._paq == undefined) {
      window._paq = [];
    }
    let _paq = window._paq;
    /* tracker methods like "setCustomDimension" should be called before "trackPageView" */
    _paq.push(['trackPageView']);
    if (MATOMO_ENABLE_LINK_TRACKING) {
      _paq.push(['enableLinkTracking']);
    }
    (function() {
      var u=MATOMO_TRACKER_URL;
      _paq.push(['setTrackerUrl', u+'piwik.php']);
      _paq.push(['setSiteId', MATOMO_SITE_ID]);
      var d=document, g=d.createElement('script'), s=d.getElementsByTagName('script')[0];
      g.type='text/javascript'; g.async=true; g.defer=true; g.src=u+'piwik.js'; s.parentNode.insertBefore(g,s);
    })();
    router.afterEach(function (to) {
      // Use window global here.
      window._paq.push(['trackPageView', to.fullPath]);
    });
  }
}
