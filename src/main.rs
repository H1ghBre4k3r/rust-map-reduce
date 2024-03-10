use std::{collections::HashMap, error::Error, hash::Hash};

use tokio::{fs, spawn};

struct Framework<I> {
    input: Vec<I>,
}

impl<I> Framework<I>
where
    I: Send + Clone + 'static,
{
    fn new(input: Vec<I>) -> Self {
        Framework { input }
    }

    async fn run<M, K, O, R>(&mut self, mapper: M, reducer: R)
    where
        M: Fn(I) -> (K, O) + Send + Copy + 'static,
        K: Eq + Hash + Clone + Send + 'static,
        R: Fn(K, Vec<O>) + Send + Copy + 'static,
        O: Clone + Send + 'static,
    {
        let mapped = self.map(mapper).await;
        self.reduce(mapped, reducer).await;
    }

    async fn map<M, K, O>(&mut self, mapper: M) -> Vec<(K, O)>
    where
        M: Fn(I) -> (K, O) + Send + Copy + 'static,
        K: Send + 'static,
        O: Clone + Send + 'static,
    {
        let mut tasks = vec![];

        for i in self.input.iter() {
            let i = i.clone();
            tasks.push(spawn(async move { mapper(i) }));
        }

        let mut mapped = vec![];

        for task in tasks.into_iter() {
            match task.await {
                Ok(res) => mapped.push(res),
                Err(e) => eprintln!("Error during map: {e:?}"),
            }
        }

        mapped
    }

    async fn reduce<'a, K, O, R>(&mut self, mapped: Vec<(K, O)>, reducer: R)
    where
        K: Eq + Hash + Clone + Send + 'static,
        R: Fn(K, Vec<O>) + Send + Copy + 'static,
        O: Clone + Send + 'static,
    {
        let mut sections = HashMap::new();
        for (key, val) in mapped.iter() {
            let mut values = sections.get(key).cloned().unwrap_or(vec![]);
            values.push(val.clone());
            sections.insert(key.clone(), values);
        }

        let mut tasks = vec![];

        for (key, values) in sections {
            tasks.push(spawn(async move {
                reducer(key, values);
            }));
        }

        for task in tasks.into_iter() {
            if let Err(e) = task.await {
                eprintln!("Error: {e:?}");
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let input = fs::read_to_string("input.txt").await?;

    let input = input
        .split_whitespace()
        .map(|i| i.to_string())
        .collect::<Vec<_>>();

    let mut framework = Framework::new(input);

    framework
        .run(
            |val| {
                let val = val.parse::<i64>().unwrap();
                (val % 2 == 0, val)
            },
            |key, values| println!("{key}: {}", values.iter().sum::<i64>()),
        )
        .await;

    Ok(())
}
