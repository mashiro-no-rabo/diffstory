mod template;

use crate::github::PrInfo;
use crate::matcher::ResolvedStory;

pub fn render(story: &ResolvedStory, title: Option<&str>, author: Option<&str>, pr_info: Option<&PrInfo>) -> String {
  template::render(story, title, author, pr_info)
}
