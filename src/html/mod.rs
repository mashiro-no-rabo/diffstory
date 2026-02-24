mod template;

use crate::matcher::ResolvedStory;

pub fn render(story: &ResolvedStory, title: Option<&str>, author: Option<&str>) -> String {
  template::render(story, title, author)
}
