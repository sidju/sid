fn fizzbuzz(i: i64) {
  if i.modulo(15) == 0 {
    println!("fizzbuzz");
  }
  else if i.modulo(5) == 0 {
    println!("buzz");
  }
  else if i.modulo(3) == 0 {
    println!("fizz");
  }
  else {
    println!("{}", i);
  }
}
fn alt_fizzbuzz(i: i64) {
  match i.modulo(15) {
    0 => println!("fizzbuzz"),
    5 | 10 => println!("buzz"),
    3 | 6 | 9 | 12 => println!("fizz"),
    _ => println!("{i}"),
  }
}
fn main() {
  for i in [1..100] {
    fizzbuzz(i);
  }
}
