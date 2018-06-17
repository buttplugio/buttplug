module.exports = {
  "root": "./protocol-spec",
  "plugins": [],
  "pluginsConfig": {}
};

module.exports["plugins"].push("mermaid-gb3");
module.exports["pluginsConfig"]["mermaid-gb3"] = {
  "sequenceDiagram": {
    "actorMargin": 200
  }
};

// Only add piwik if we're building on the CI and deploying
if (process.env.CI) {
  module.exports["plugins"].push("piwik");
  module.exports["pluginsConfig"] = {
    "piwik": {
      "URL": "matomo.nonpolynomial.com/",
      "siteId": 7,
      "phpPath": "js/",
      "jsPath": "js/"
    }
  };
}
