fn main() {
  prost_build::compile_protos(
    &[
      "src/device/protocol/thehandy/protocomm.proto",
      "src/device/protocol/thehandy/handyplug.proto",
    ],
    &["src/device/protocol/thehandy"],
  )
  .expect("These will always compile or we shouldn't be building.");
}
