module surface
import core.Option
import core.Result

struct Pair<T> {
  left: T,
  right: T
}

enum State<T> {
  Ready(T),
  Empty
}

trait Describe {
  fn describe(value: i32) -> String;
}

impl Describe for i32 {
  fn describe(value: i32) -> String {
    return "state"
  }
}

fn wrap<T>(value: T) -> Option<T> {
  return Option.Some(value)
}

fn choose(value: i32) -> Result<i32, String> {
  let candidate: Option<i32> = wrap(value)
  return Result.Ok(value)
}

fn main(value: i32) -> Result<i32, String> {
  let parsed: i32 = choose(value)?
  if parsed > 0 { Result.Ok(parsed) } else { Result.Ok(0) }
}
