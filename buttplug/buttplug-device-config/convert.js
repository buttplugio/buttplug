const yaml = require('js-yaml');
const fs   = require('fs');

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
    featureObj["actuator"]["messages"] = ["RotateCmd", "ScalarCmd"];
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
const doc = yaml.load(fs.readFileSync('./buttplug-device-config.yml', 'utf8'));
for (var protocol in doc["protocols"]) {
  console.log(protocol);
  if (doc["protocols"][protocol]["defaults"] === undefined) {
    console.log("No defaults for protocol");
  }
  if (doc["protocols"][protocol]["defaults"]["messages"] !== undefined) {
    doc["protocols"][protocol]["defaults"]["features"] = convertMessagesObject(doc["protocols"][protocol]["defaults"]["messages"]);
    doc["protocols"][protocol]["defaults"]["messages"] = undefined;
  }
  if (doc["protocols"][protocol]["configurations"] !== undefined) {
    for (var config of doc["protocols"][protocol]["configurations"]) {
      if (config["messages"] !== undefined) {
        config["features"] = convertMessagesObject(config["messages"]);
        config["messages"] = undefined;
      }
    }
  }
}

fs.writeFileSync("buttplug-device-config-convert.yml", yaml.dump(doc));
