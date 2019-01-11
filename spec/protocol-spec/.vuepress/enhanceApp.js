import Vue from 'vue';
//import VueMatomo from 'vue-matomo';

export default ({
  Vue, // the version of Vue being used in the VuePress app
  options, // the options for the root Vue instance
  router, // the router instance for the app
  siteData // site metadata
}) => {
  // Vue.use(VueMatomo, {
  //   // Configure your matomo server and site
  //   host: 'https://matomo.nonpolynomial.com',
  //   siteId: 7,

  //   // Enables automatically registering pageviews on the router
  //   router: router,

  //   // Enables link tracking on regular links. Note that this won't
  //   // work for routing links (ie. internal Vue router links)
  //   // Default: true
  //   enableLinkTracking: true,

  //   // Require consent before sending tracking information to matomo
  //   // Default: false
  //   requireConsent: false,

  //   // Whether to track the initial page view
  //   // Default: true
  //   trackInitialView: true,

  //   // Changes the default .js and .php endpoint's filename
  //   // Default: 'piwik'
  //   trackerFileName: 'piwik',

  //   // Whether or not to log debug information
  //   // Default: false
  //   debug: false
  // });
};
