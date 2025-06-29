const ajv = require("ajv");
const fs = require('fs').promises;
const assert = require('assert');

async function test() {
  const tests = JSON.parse(await fs.readFile("./tests/schema-test.json", "utf-8"));
  const schema = JSON.parse(await fs.readFile("./schema/buttplug-schema.json", "utf-8"));
  const validator = new ajv();
  validator.addMetaSchema(require("ajv/lib/refs/json-schema-draft-06.json"));
  const jsonValidator = validator.compile(schema);
  for (const test of tests) {
    console.log("Running " + test["Description"]);
    for (const testName of test["Tests"]) {
      if (testName === "ShouldPassParse") {
        assert(jsonValidator(test["Messages"]), jsonValidator.errors ? jsonValidator.errors.map((error) => error.message).join("; ") : "No errors");
      }
      else if (testName === "ShouldFailParse") {
        assert(!jsonValidator(test["Messages"]), `Test ${test["Description"]} passed parsing when it should have failed`);
      }
      else if (testName === "ShouldFailOnExtraField") {
        let obj = {... test["Message"]};
        obj["ExtraField"] = "I'm a useless extra field";
        assert(!jsonValidator(obj), `Test ${test["Description"]} passed with extra field when it should have failed`);
      } else {
        assert(false, `Test name ${testName} not valid`);
      }
    }
  }
}

test().then(() => { console.log("Done"); }).catch((err) => { console.log(`failure: ${err}`); });
