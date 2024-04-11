const yaml = require('js-yaml');
const fs   = require('fs');

function moveDefaults(def, config) {
  if (def["ScalarCmd"] !== undefined && config["ScalarCmd"] === undefined) {
    config["ScalarCmd"] = JSON.parse(JSON.stringify(def["ScalarCmd"]))
  }
  if (def["RotateCmd"] !== undefined && config["RotateCmd"] === undefined) {
    config["RotateCmd"] = JSON.parse(JSON.stringify(def["RotateCmd"]))
  }
  if (def["LinearCmd"] !== undefined && config["LinearCmd"] === undefined) {
    config["LinearCmd"] = JSON.parse(JSON.stringify(def["LinearCmd"]))
  }
  if (def["SensorReadCmd"] !== undefined && config["SensorReadCmd"] === undefined) {
    config["SensorReadCmd"] = JSON.parse(JSON.stringify(def["SensorReadCmd"]))
  }
}

function convertMessagesObject(messages) {
  let features = [];
  console.log(messages["ScalarCmd"]);
  if (messages["ScalarCmd"] !== undefined) {
  for (var scalarcmd of messages["ScalarCmd"]) {
    let featureObj = {};
    console.log(scalarcmd);
    featureObj["feature-type"] = scalarcmd["ActuatorType"];
    if (scalarcmd["FeatureDescriptor"] !== undefined) {
      featureObj["description"] = scalarcmd["FeatureDescriptor"];
    }
    featureObj["actuator"] = {};
    if (scalarcmd["StepRange"] !== undefined) {
      featureObj["actuator"]["step-range"] = scalarcmd["StepRange"];
    }
    featureObj["actuator"]["messages"] = ["ScalarCmd"];
    features.push(featureObj);
  }
}
if (messages["RotateCmd"] !== undefined) {
  for (var scalarcmd of messages["RotateCmd"]) {
    let featureObj = {};
    console.log(scalarcmd);
    featureObj["feature-type"] = scalarcmd["ActuatorType"];
    if (scalarcmd["FeatureDescriptor"] !== undefined) {
      featureObj["description"] = scalarcmd["FeatureDescriptor"];
    }
    featureObj["actuator"] = {};
    if (scalarcmd["StepRange"] !== undefined) {
      featureObj["actuator"]["step-range"] = scalarcmd["StepRange"];
    }
    featureObj["actuator"]["messages"] = ["RotateCmd"];
    features.push(featureObj);
  }
}
if (messages["LinearCmd"] !== undefined) {
  for (var scalarcmd of messages["LinearCmd"]) {
    let featureObj = {};
    console.log(scalarcmd);
    featureObj["feature-type"] = scalarcmd["ActuatorType"];
    if (scalarcmd["FeatureDescriptor"] !== undefined) {
      featureObj["description"] = scalarcmd["FeatureDescriptor"];
    }
    featureObj["actuator"] = {};
    if (scalarcmd["StepRange"] !== undefined) {
      featureObj["actuator"]["step-range"] = scalarcmd["StepRange"];
    }
    featureObj["actuator"]["messages"] = ["LinearCmd"];
    features.push(featureObj);
  }
}
if (messages["SensorReadCmd"] !== undefined) {
  for (var sensorcmd of messages["SensorReadCmd"]) {
    let featureObj = {};
    console.log(scalarcmd);
    featureObj["feature-type"] = sensorcmd["SensorType"];
    if (sensorcmd["FeatureDescriptor"] !== undefined) {
      featureObj["description"] = sensorcmd["FeatureDescriptor"];
    }
    featureObj["sensor"] = {};
    if (sensorcmd["SensorRange"] !== undefined) {
      featureObj["sensor"]["value-range"] = sensorcmd["SensorRange"];
    }
    featureObj["sensor"]["messages"] = ["SensorReadCmd"];
    features.push(featureObj);
  }
}
  return features;
}

// Get document, or throw exception on error
const doc = yaml.load(fs.readFileSync('./device-config-v2/buttplug-device-config.yml', 'utf8'));
for (var protocol in doc["protocols"]) {
  console.log(protocol);
  if (doc["protocols"][protocol]["defaults"] === undefined) {
    console.log("No defaults for protocol");
  }
  let def = undefined;
  if (doc["protocols"][protocol]["defaults"]["messages"] !== undefined) {
    def = doc["protocols"][protocol]["defaults"]["messages"];
    doc["protocols"][protocol]["defaults"]["features"] = convertMessagesObject(doc["protocols"][protocol]["defaults"]["messages"]);
    doc["protocols"][protocol]["defaults"]["messages"] = undefined;
  }
  if (doc["protocols"][protocol]["configurations"] !== undefined) {
    for (var config of doc["protocols"][protocol]["configurations"]) {
      if (config["messages"] !== undefined) {
        if (def !== undefined) {
          moveDefaults(def, config["messages"])
        }
        config["features"] = convertMessagesObject(config["messages"]);
        config["messages"] = undefined;
      }
    }
  }
}

fs.writeFileSync("device-config-v3/buttplug-device-config-v3.yml", yaml.dump(doc));
