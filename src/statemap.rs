extern crate serde;
extern crate serde_derive;
extern crate serde_json;

use serde::{Serialize, Deserialize};

use chrono::{Datelike, Timelike, NaiveDate};

use std::str::FromStr;
use std::iter::Iterator;
use std::iter::IntoIterator;

use std::collections::{HashMap, LinkedList};
use std::collections::hash_map::IntoIter;

/*
 * The Statemap* types denote the structure of the JSON that statemap expects.
 * This is the definition of the statemap 'on disk format' of sorts.
 */
#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct StatemapState {
    color: Option<String>,                  // color for state, if any
    value: usize,                           // value for state
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct StatemapDatum {
    #[serde(deserialize_with = "datum_time_from_string")]
    #[serde(serialize_with = "datum_string_from_time")]
    time: u64,                              // time of this datum
    entity: String,                         // name of entity
    state: u32,                             // state entity is in at time
    tag: Option<String>,                    // tag for this state, if any
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct StatemapDescription {
    entity: String,                         // name of entity
    description: String,                    // description of entity
}

#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
#[serde(deny_unknown_fields)]
pub struct StatemapMetadata {
    start: Vec<u64>,
    title: String,
    host: Option<String>,
    entityKind: Option<String>,
    states: HashMap<String, StatemapState>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct StatemapEvent {
    time: String,                           // time of this datum
    entity: String,                         // name of entity
    event: String,                          // type of event
    target: Option<String>,                 // target for event, if any
}

#[derive(Deserialize, Debug)]
pub struct StatemapTag {
    state: u32,                             // state for this tag
    tag: String,                            // tag itself
}

/*
 * The time value is written in the input as a JSON string containing a number.
 * Deserialize just the number here without allocating memory for a String.
 */
fn datum_time_from_string<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: &str = serde::Deserialize::deserialize(deserializer)?;
    match u64::from_str(s) {
        Ok(time) => Ok(time),
        Err(_) => Err(serde::de::Error::custom("illegal time value")),
    }
}

/*
 * The opposite of datum_time_from_string, this function changes the given u64
 * timestamp into a string so it can be stored in JSON (which can't hold 64 bit
 * numbers natively).
 */
fn datum_string_from_time<S>(time: &u64, serializer: S)
    -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&format!("{}", time))
}

pub struct Statemap {
    metadata: StatemapMetadata,
    state_data: HashMap<String, LinkedList<StatemapDatum>>,
    first_state: Option<u64>,
}

/*
 * Consumers of Statemap will use an iterator to pull the state information out
 * of this library. The iterator consumes the Statemap struct.
 *
 * The Statemap is consumed primarily because the intended use of this library
 * in its current state is to help users create a singular statemap. This
 * library does not currently support something like a 'streaming statemap'
 * because the statemap tool does not support this. The statemap tool requires
 * a rigid period of time for rendering.
 * 
 * Adding new data points after or while iterating over the Statemap could
 * cause unexpected results. The statemap protocol requires that a header is
 * provided that includes the beginning time (in UTC, but this is hard to
 * enforce) of the statemap. All state datum provide offsets from this start
 * time.
 *
 * This library abstracts this concept away as much as possible and
 * calculates the start time on-the-fly. If the consumer adds earlier states
 * to the Statemap during or after iteration the results can be confusing.
 *
 */
impl Statemap {
    pub fn new(title: &str, host: Option<String>, entity_kind: Option<String>)
        -> Statemap {

        Statemap {
            metadata: StatemapMetadata {
                start: Vec::new(),
                title: title.to_owned(),
                host,
                entityKind: entity_kind,
                states: HashMap::new(),
            },
            state_data: HashMap::new(),
            first_state: None,
        }
    }

    /*
     * Sets the given entity to the given state.
     *
     * If the given state is not already registered in the statemap metadata it
     * is registered here without a color. The statemap tool will assign a
     * random color for the state when it creates the SVG.
     *
     * I'm not sure of a way to enforce the statemap tool's recommendation for a
     * UTC-based time zone, so we just require a Datelike+Timelike
     * implementation. Hopefully users are aware of the UTC recommendation, or
     * don't care if wall clock times aren't accurate.
     */
    pub fn set_state<D>(&mut self, entity_name: &str, state_name: &str,
        tag: Option<&str>, datetime: D)
    where
        D: Datelike + Timelike,
    {

        let ename = entity_name.to_owned();
        let sname = state_name.to_owned();
        let mut t: Option<String> = None;
        if tag.is_some() {
            t = Some(tag.unwrap().to_owned());
        }

        let len = self.metadata.states.len();
        let state = self.metadata.states
            .entry(sname)
            .or_insert(StatemapState {
                color: None,
                value: len,
            });

        let hr = datetime.hour();
        let min = datetime.minute();
        let sec = datetime.second();
        let ns: u64 = datetime.nanosecond() as u64;
        let yr = datetime.year();
        let mon = datetime.month();
        let day = datetime.day();

        let time = NaiveDate::from_ymd(yr, mon, day).and_hms(hr, min, sec);
        let mut ts: u64 = (time.timestamp() as u64)* 1_000_000_000;
        ts += ns;

        if self.first_state.is_none() || self.first_state.unwrap() > ts {
            self.first_state = Some(ts);
        }

        let datum = StatemapDatum {
            time: ts,
            entity: ename.clone(),
            state: state.value as u32,
            tag: t,
        };

        self.state_data
            .entry(ename)
            .and_modify(|e| e.push_back(datum.clone()))
            .or_insert_with(|| {
                let mut list = LinkedList::new();
                list.push_back(datum);
                list
            });
    }
}

/*
 * Iterator state.
 *
 * We need to iterate over each of the entities in the hash map and all of the
 * states for each entity.
 *
 */
pub struct IterHelper {
    header: StatemapMetadata,
    first_state: Option<u64>,
    entity_iter: IntoIter<String, LinkedList<StatemapDatum>>,
    entity_data: Option<(String, LinkedList<StatemapDatum>)>,
}

impl IterHelper {
    /*
     * Statemaps require a header that includes a start time array. The array
     * must have two elements.
     *   start[0] -> Seconds since Unix epoch in UTC.
     *   start[1] -> Nanosecond offset within the time defined by start[0].
     */
    fn update_header(&mut self) {
        let sec = self.first_state.unwrap() / 1_000_000_000;
        let ns = self.first_state.unwrap() % 1_000_000_000;

        /*
         * Replace the vec in the header with the new start time.
         */
        self.header.start = vec![sec, ns];
    }
}

impl Iterator for IterHelper {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let mut ret = None;

        /*
         * The beginning of the iterator prints the statemap header data.
         *
         * We need to make sure the header is configured with the correct
         * start time before returning the formatted JSON.
         */
        if self.entity_data.is_none() {
            self.entity_data = self.entity_iter.next();

            self.update_header();
            return Some(serde_json::to_string(&self.header).unwrap())
        }

        /*
         * TODO this should really be a layered into_iter() for the LinkedList,
         * but that is difficult to accomplish.
         *
         * The LinkedList and Vec into_iter implementations that get applied
         * here are for referenced data since the trait definition for this
         * function makes us use an &mut reference (IIUC). I'm not aware of a
         * way to enforce lifetimes, of which there could be three, for this
         * type of work.
         *
         * The problem is either my comprehension of lifetimes and references
         * (likely), or a problem with multiple layered iterators and the trait
         * definition of IntoIterator/Iterator (less likely).
         *
         * In any case, this is some code that could be a lot cleaner if we
         * could use two iterators instead of one iterator and by-hand
         * LinkedList iteration.
         */
        loop {
            if let Some((_, statelist)) = &mut self.entity_data {
                ret = match statelist.pop_front() {
                    Some(mut state) => {
                        state.time -= self.first_state.unwrap();

                        Some(serde_json::to_string(&state).unwrap())
                    },
                    None => None,
                }
            }

            if ret.is_some() {
                break;
            }

            if ret.is_none() {
                self.entity_data = self.entity_iter.next();

                if self.entity_data.is_none() {
                    break;
                }
            }
        }

        ret
    }
}

impl IntoIterator for Statemap {
    type Item = String;
    type IntoIter = IterHelper;

    fn into_iter(self) -> Self::IntoIter {
        IterHelper {
            header: self.metadata,
            first_state: self.first_state,
            entity_iter: self.state_data.into_iter(),
            entity_data: None,
        }
    }
}
