

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HumsafaarUsageType {
  TextToResult,
  ImageToResult,
  Other,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HumsafaarUsagePage {
  ImagePage,
  VideoPage,
  EditPage,
  StagePage,
  ObjectPage,
  OtherPage,
}
