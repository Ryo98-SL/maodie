module demo
import core.Option
import core.Result

struct Box<T> {
  value: T
}

enum Choice {
  Small,
  Large
}

trait Score {
  fn score(value: i32) -> String;
}

impl Score for i32 {
  fn score(value: i32) -> String {
    return "score"
  }
}

fn identity<T>(value: T) -> T {
  return value
}

fn classify(value: i32) -> Choice {
  if value > 10 { Choice.Large } else { Choice.Small }
}

fn parse(value: i32) -> Result<i32, String> {
  let mut shifted: i32 = value + 1
  shifted = shifted + 1

  match classify(shifted) {
    Choice.Small => Result.Ok(identity(shifted)),
    Choice.Large => Result.Ok(shifted + 10)
  }
}

fn main(value: i32) -> Result<i32, String> {
  let parsed: i32 = parse(value)?
  let maybe: Option<i32> = Option.Some(parsed)
  let score: i32 = parsed

  return Result.Ok(score)
}
