export type WorkbenchExampleId = "hello" | "functions" | "fibonacci" | "v1";

export interface WorkbenchExample {
  readonly id: WorkbenchExampleId;
  readonly label: string;
  readonly description: string;
  readonly source: string;
}

const helloWorldSource = `module demo
import core.Result
import core.log

fn hello() -> String {
  return "Hello world"
}

fn main(value: i32) -> Result<i32, String> {
  log("Hello world")
  return Result.Ok(value)
}
`;

const functionCallSource = `module demo
import core.Result

fn double(value: i32) -> i32 {
  return value * 2
}

fn add(left: i32, right: i32) -> i32 {
  return left + right
}

fn main(value: i32) -> Result<i32, String> {
  let doubled: i32 = double(value)
  return Result.Ok(add(doubled, 3))
}
`;

const fibonacciSource = `module demo
import core.Result

fn fib(value: i32) -> i32 {
  if value < 2 { value } else { fib(value - 1) + fib(value - 2) }
}

fn main(value: i32) -> Result<i32, String> {
  return Result.Ok(fib(value))
}
`;

export const defaultSource = `module demo
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
`;

export const defaultExampleId: WorkbenchExampleId = "v1";

export const workbenchExamples: readonly WorkbenchExample[] = [
  {
    id: "hello",
    label: "Hello World",
    description: "最小模块、字符串字面量和 Result 返回。",
    source: helloWorldSource
  },
  {
    id: "functions",
    label: "函数调用",
    description: "定义多个函数并在 main 中组合调用。",
    source: functionCallSource
  },
  {
    id: "fibonacci",
    label: "斐波那契",
    description: "用递归和 if 表达式计算 fib(n)。",
    source: fibonacciSource
  },
  {
    id: "v1",
    label: "V1 综合",
    description: "覆盖当前 v1 acceptance 的核心语法面。",
    source: defaultSource
  }
] as const;
