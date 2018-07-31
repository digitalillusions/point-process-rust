/*!
This module implements a set of time-dependent point processes, such as Poisson or Hawkes processes, on the real half-line [0,∞[.
*/

use rand::prelude::*;
use rand::distributions::Uniform;
use rand::distributions::Poisson;

use rayon::prelude::*;
use std::sync::Arc;

use event::Event;

/// Simulate a homogeneous, constant-intensity Poisson process.
pub fn poisson_process(tmax: f64, lambda: f64) -> Vec<Event> {
    /// A poisson process cannot have negative intensity.
    assert!(lambda >= 0.0);
    let mut rng = thread_rng();
    let num_events = Poisson::new(tmax*lambda).sample(&mut rng);
    
    (0..num_events).into_par_iter().map(|_| {
        // get reference to local thread rng
        let mut rng = thread_rng();
        let u = Uniform::new(0.0, tmax);
        let timestamp = u.sample(&mut rng);
        Event::new(timestamp, lambda)
    }).collect()
}

/// Simulate a Poisson process with variable intensity.
pub fn variable_poisson<F>(tmax: f64, lambda: F, max_lambda: f64) -> Vec<Event>
where F: Fn(f64) -> f64 + Send + Sync
{
    // Number of events before thinning
    let mut rng = thread_rng();
    let num_events = Poisson::new(tmax*max_lambda).sample(&mut rng);

    let lambda = Arc::from(lambda);

    // Get timestamp and intensity values of events distributed
    // according to a homogeneous Poisson process
    // and keep those who are under the intensity curve
    (0..num_events).into_par_iter().filter_map(|_| {
        let mut rng = thread_rng();
        let u = Uniform::new(0.0, 1.0);
        let timestamp = {
            tmax*u.sample(&mut rng)
        };
        let lambda_val = {
            max_lambda*u.sample(&mut rng)
        };

        if lambda_val < lambda(timestamp) {
            let event = Event::new(timestamp, lambda_val);
            Some(event)
        } else {
            None
        }
    }).collect()
}

/// Simulate a Hawkes process with an exponential kernel
/// by utilising the linear time-complexity algorithm in [Dassios and Zhao's 2013 paper](http://eprints.lse.ac.uk/51370/1/Dassios_exact_simulation_hawkes.pdf).
/// Returns the intensity process.
/// This will borrow and consume the given `jumps` iterator, and will panic if it turns up empty.
pub fn hawkes_exponential<T>(tmax: f64, beta: f64, lambda0: f64, jumps: &mut T) -> Vec<Event>
where T: Iterator<Item = f64>
{
    let mut t = 0.0;
    let mut previous_t: f64;
    let mut last_lambda = lambda0;
    let mut result = Vec::<Event>::new();

    while t < tmax {
        // variables U_1 and S_{k+1}^(1) from the paper @DassiosZhao13
        let u1 = random::<f64>();
        let s1 = -1.0/beta*u1.ln();

        let d = if last_lambda > lambda0 {
            1.0 + beta*u1.ln()/(last_lambda - lambda0)
        } else { ::std::f64::NEG_INFINITY };
        
        let s2 = -1.0/lambda0*random::<f64>().ln();
        
        previous_t = t;

        t += if d < 0.0 {
            s2
        } else {
            s1.min(s2)
        };

        last_lambda = lambda0 + (last_lambda - lambda0)*(-beta*(t-previous_t)).exp();

        // Try to get a jump value, create an `Event` instance (and set the jump to it) and add it to our result object.
        if let Some(alpha) = jumps.next() {
            let new_event = Event::new(t, last_lambda).jump(alpha);
            result.push(new_event);
            last_lambda += alpha;
        } else {
            panic!("Hawkes process ran out of jumps!");
        }

    }

    result
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_serialize() {
        let event = Event::new(42.0, 15.02);

        let event_serialized = serde_json::to_string_pretty(&event).unwrap();

        println!("{}", event_serialized);
    }
}
