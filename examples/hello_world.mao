module demo
import core.Result
import core.log

fn main(value: i32) -> Result<i32, String> {
  log("Hello world")
  return Result.Ok(value)
}
