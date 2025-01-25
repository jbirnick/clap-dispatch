use clap::{Args, Parser};
use clap_dispatch::clap_dispatch;

#[derive(Parser)]
#[clap_dispatch(fn sort(self, nums: Vec<i32>) -> Vec<i32>)]
enum Cli {
    Quick(QuickArgs),
    Merge(MergeArgs),
}

#[derive(Args)]
struct QuickArgs;

#[derive(Args)]
struct MergeArgs {
    #[arg(long)]
    in_place: bool,
}

impl Sort for QuickArgs {
    fn sort(self, nums: Vec<i32>) -> Vec<i32> {
        // in reality would need to implement sorting algorithm here
        println!("Running quicksort!");
        nums
    }
}

impl Sort for MergeArgs {
    fn sort(self, nums: Vec<i32>) -> Vec<i32> {
        // in reality would need to implement sorting algorithm here
        match self.in_place {
            false => println!("Running mergesort out-of-place!"),
            true => println!("Running mergesort in-place!"),
        }
        nums
    }
}

fn main() {
    let algo = Cli::parse();

    // in reality would read the numbers from a file or so
    let nums = Vec::new();

    // simple dispatch thanks to the macro!
    algo.sort(nums);
}
