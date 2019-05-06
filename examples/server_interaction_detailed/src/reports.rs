#[macro_use]
extern crate seed;
use seed::prelude::*;

use futures::Future;
use serde::Deserialize;

mod interfaces;
use crate::interfaces::{Gradesheet, Line, Mission, Person, Syllabus, Upgrade, UpgradeEvent};
mod util;

use chrono::offset::{TimeZone, Utc};
use chrono::{Date, Duration};

struct Model {
    lines: Vec<Line>,
    people: Vec<Person>,
    syllabi: Vec<Syllabus>,
    upgrades: Vec<Upgrade>,
    events: Vec<UpgradeEvent>,
    gradesheets: Vec<Gradesheet>,
    instructors: Vec<Person>,
    missions: Vec<Mission>,
    mx_start: Date<Utc>,
    mx_end: Date<Utc>,
}

impl Default for Model {
    fn default() -> Self {
        // todo way to take adv of Default::default for all but dates to reduce clutter?
        Self {
            lines: Vec::new(),
            people: Vec::new(),
            syllabi: Vec::new(),
            upgrades: Vec::new(),
            events: Vec::new(),
            gradesheets: Vec::new(),
            instructors: Vec::new(),
            missions: Vec::new(),
            mx_start: chrono::Utc.ymd(1999, 9, 9),
            mx_end: chrono::Utc.ymd(1999, 9, 9),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
struct ServerData {
    lines: Vec<Line>,
    people: Vec<Person>,

    syllabi: Vec<Syllabus>,
    upgrades: Vec<Upgrade>,
    events: Vec<UpgradeEvent>,
    gradesheets: Vec<Gradesheet>,
    instructors: Vec<Person>,
    missions: Vec<Mission>,
}

fn get_data() -> impl Future<Item = Msg, Error = Msg> {
    let url = "/get-reports-data/";

    seed::Request::new(url)
        .method(seed::Method::Get)
        .fetch_json()
        .map(Msg::LoadInitial)
        .map_err(Msg::OnFetchErr)
}

#[derive(Clone)]
enum Msg {
    GetData,
    LoadInitial(ServerData),
    OnFetchErr(JsValue),

    ChangeMxStart(chrono::Date<Utc>),
    ChangeMxEnd(chrono::Date<Utc>),
}

fn update(msg: Msg, model: &mut Model) -> Update<Msg> {
    match msg {
        Msg::GetData => Update::with_future_msg(get_data()).skip(),

        Msg::LoadInitial(data) => {
            model.lines = data.lines;
            model.people = data.people;
            model.missions = data.missions;

            Render.into()
        }

        Msg::OnFetchErr(err) => {
            log!(format!("Fetch error: {:?}", err));
            Skip.into()
        }

        Msg::ChangeMxStart(date) => {
            model.mx_start = date;
            Render.into()
        }
        Msg::ChangeMxEnd(date) => {
            model.mx_end = date;
            Render.into()
        }
    }
}

fn gradesheets(model: &Model) -> El<Msg> {
    section![]
}

fn mx_effectivity(lines: &Vec<Line>, mx_start: &Date<Utc>, mx_end: &Date<Utc>) -> El<Msg> {
    section![
        style! {"margin-bottom" => 100},
        h2!["Maintenance line effectivity"],
        div![
            style! {"display" => "flex"; "flex-direction" => "column"},
            div![
                style! {"display" => "flex"; "margin-bottom" => 60},
                h3!["Start"],
            ]
        ]
    ]
}

fn display_pct<T>(part: &Vec<T>, whole: &Vec<T>) -> String {
    if whole.len() > 0 {
        (100 * part.len() / whole.len()).to_string()
    } else {
        "0".into()
    }
}

fn display_pct2<T>(len: usize, whole: &Vec<T>) -> String {
    // Alternate variant.
    if whole.len() > 0 {
        (100 * len / whole.len()).to_string()
    } else {
        "0".into()
    }
}

fn sortie_types(lines: &Vec<Line>, people: &Vec<Person>, missions: &Vec<Mission>) -> El<Msg> {
    let lookback_days = 60; // todo make adjustable
                            //    let min_date = Utc::today() - Duration::days(lookback_days);
                            //    let today = Utc::today();

    let mut rows: Vec<El<Msg>> = Vec::new();

    for person in people.iter().filter(|p| util::is_aircrew(*p)) {
        // todo person_lines is much too verbose.

        let person_lines: Vec<&Line> = lines
            .into_iter()
            .filter(|l| {
                // todo handle l_date in deser
                //                let l_date = Utc
                //                    .datetime_from_str(&l.date, "%Y-%m-%d")
                //                    .expect("Can't format date")
                //                    .date();

                if let Some(pilot_id) = l.pilot {
                    let pilot = people.iter().find(|p| p.id == pilot_id);
                    //                        .expect("Can't find pilot");
                    //                if pilot.id == person.id && min_date <= l_date && l_date <= today {
                    if let Some(pilot) = pilot {
                        if pilot.id == person.id {
                            return true;
                        }
                    }
                }
                // todo DRY
                if let Some(wso_id) = l.wso {
                    let wso = people.iter().find(|p| p.id == wso_id);
                    //                        .expect("Can't find WSO");
                    //                if wso.id == person.id && min_date <= l_date && l_date <= today {
                    if let Some(wso) = wso {
                        if wso.id == person.id {
                            return true;
                        }
                    }
                }
                false
            })
            .collect();

        let red_air: Vec<&Line> = person_lines
            .clone() // todo sloppy clone
            .into_iter()
            .filter(|l| {
                if let Some(mission_id) = l.mission2 {
                    let mission = missions
                        .iter()
                        .find(|m| m.id == mission_id)
                        .expect("Can't find mission");
                    if mission.name.to_lowercase() == "red air" {
                        return true;
                    }
                }
                false
            })
            .collect();

        let mut upgrade: Vec<&Line> = Vec::new();
        for line in person_lines.clone() {
            // todo sloppy clone
            let mut found_one = false;
            for form_line in util::formation_lines(line, lines) {
                if form_line.upgrade || form_line.wso_upgrade || form_line.upgrade_event.is_some() {
                    upgrade.push(line);
                    found_one = true;
                    break;
                }
            }
            if found_one {
                break;
            }
        }

        rows.push(tr![
            style! {"text-align" => "center"},
            td![style! {"text-align" => "left"}, util::short_name(person)],
            td![display_pct(&red_air, &person_lines)],
            td![display_pct(&upgrade, &person_lines)],
            td![display_pct2(
                &person_lines.len() - &red_air.len() - &upgrade.len(),
                &person_lines
            )],
        ]);
    }

    section![
        style! {"margin-bottom" => 100},
        h2!["Sortie types (last 60 days)"],
        table![
            class![
                "table",
                "table-striped",
                "table-condensed",
                "table-responsive"
            ],
            thead![tr![
                th!["Person"],
                th!["% Red air"],
                th!["% Upgrade"],
                th!["% CT"],
            ]],
            tbody![rows],
        ]
    ]
}

fn view(model: &Model) -> Vec<El<Msg>> {
    vec![
        h2!["(WIP below)"],
        gradesheets(model),
        mx_effectivity(&model.lines, &model.mx_start, &model.mx_end),
        sortie_types(&model.lines, &model.people, &model.missions),
    ]
}

#[wasm_bindgen]
pub fn render() {
    let state = seed::App::build(Model::default(), update, view)
        .finish()
        .run();

    state.update(Msg::GetData)
}