use smol_str::SmolStr;

pub fn lcp_of_array(arr: &Vec<SmolStr>) -> String {
  assert!(arr.len() > 0);
  if arr.len() == 1 {
    return arr[0].to_string();
  }

  let mut longest = arr[0].clone();
  for i in 1..arr.len() {
    longest = lcp(longest.as_str(), arr[i].as_str()).into();
  }
  longest.to_string()
}
pub fn lcp<'a>(a: &'a str, b: &'a str) -> &'a str {
  let mut len = 0;
  for (l, r) in a.chars().zip(b.chars()) {
    if l != r {
      break;
    } else {
      len += 1;
    }
  }
  &a[0..len]
}
