const yaml = require('js-yaml');
const uuid = require('uuid');
const fs = require('fs');

// Get document, or throw exception on error
const doc = yaml.load(fs.readFileSync('./device-config-v4/buttplug-device-config-v4.yml', 'utf8'));
for (var protocol in doc["protocols"]) {
  console.log(protocol);
  if (doc["protocols"][protocol]["defaults"] !== undefined) {
    if (doc["protocols"][protocol]["defaults"]["id"] === undefined) {
      doc["protocols"][protocol]["defaults"]["id"] = uuid.v4();
    }
    for (var feature of doc["protocols"][protocol]["defaults"]["features"]) {
      if (feature["id"] === undefined) {
        feature["id"] = uuid.v4();
      }
    }
  }
  if (doc["protocols"][protocol]["configurations"] !== undefined) {
    for (var config of doc["protocols"][protocol]["configurations"]) {
      if (config["id"] === undefined) {
        config["id"] = uuid.v4();
      }
      if (config["features"] !== undefined) {
        for (var feature of config["features"]) {
          if (feature["id"] === undefined) {
            feature["id"] = uuid.v4();
          }
        }
      }
    }
  }
}

fs.writeFileSync("device-config-v4/buttplug-device-config-v4.yml", yaml.dump(doc));
