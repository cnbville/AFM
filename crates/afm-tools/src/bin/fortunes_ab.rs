//! Program fortunes measuring pass: one recruiting cycle replayed from a
//! saved world, so two builds of the engine sign the same recruits and
//! differ only by what changed between them.
//! `fortunes_ab <world file> <days> <out prefix>` plays the days on the
//! stand-in and writes `<prefix>.programs.csv` (each FBS program's head
//! coach, his recruiting as a recruit feels it, and its newest class: the
//! services' class score, blue-chip points, signees, their mean rating and
//! the best ten's, and the in-state ones) and `<prefix>.recruits.csv`
//! (every 3-star or better of the newest class, his rating and where he
//! signed). `docs/design/program-fortunes.md`, "Why the size of the bond
//! was not the lever", was read from these.

use afm_engine::world::data::Division;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut w = afm_engine::save::read_world(std::path::Path::new(&args[1])).expect("read the world");
    w.options.engine_from = Some(2100);
    let days: u32 = args[2].parse().expect("days");
    for _ in 0..days {
        w.advance_day();
    }
    let class = w.people.iter().filter(|p| p.cal.signed.is_some()).map(|p| p.class_of).max().unwrap_or(w.year);
    let mut out = String::from(
        "id,name,power,prestige_rel,hc,hc_skill,hc_recruiting,hc_first,class_score,points,five,four,signed,mean_ovr,top10_ovr,instate\n",
    );
    for (j, p) in w.programs.iter().enumerate().filter(|(_, p)| p.division == Division::FBS) {
        let hc = p.head_coach;
        let a = hc.map(|c| afm_engine::debug::coach_ability(&w.coaches[c]));
        let signees: Vec<u8> = w
            .people
            .iter()
            .filter(|q| q.class_of == class && q.cal.signed.is_some_and(|s| s.program == j))
            .map(|q| q.stars)
            .collect();
        let mut ovr: Vec<f64> = w
            .people
            .iter()
            .filter(|q| q.class_of == class && q.cal.signed.is_some_and(|s| s.program == j && !s.walk_on))
            .map(|q| q.player.overall())
            .collect();
        ovr.sort_by(|a, b| b.total_cmp(a));
        let mean = |v: &[f64]| if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / v.len() as f64 };
        let instate = w
            .people
            .iter()
            .filter(|q| q.class_of == class && q.cal.signed.is_some_and(|s| s.program == j) && q.home_state == p.state)
            .count();
        let five = signees.iter().filter(|&&s| s >= 5).count();
        let four = signees.iter().filter(|&&s| s == 4).count();
        let score = p.class_scores.last().map_or(0.0, |x| x.1);
        out += &format!(
            "{j},{},{},{:.4},{},{:.2},{:.2},{},{:.1},{},{five},{four},{},{:.2},{:.2},{instate}\n",
            p.real_name.replace(',', " "),
            p.power_conference(w.year) as u8,
            p.prestige_rel,
            hc.map_or(-1, |c| c as i64),
            hc.map_or(0.0, |c| afm_engine::world::fortunes::head_skill(&w, c)),
            a.map_or(0.0, |a| a.recruiting),
            hc.and_then(|c| w.coaches[c].job).map_or(0, |job| job.first_season()),
            score,
            3 * five + four,
            signees.len(),
            mean(&ovr),
            mean(&ovr[..ovr.len().min(10)])
        );
    }
    std::fs::write(format!("{}.programs.csv", args[3]), out).expect("write");
    let mut out = String::from("id,stars,pos,home_state,signed,offers,ovr\n");
    for p in w.people.iter().filter(|p| p.class_of == class && p.stars >= 3) {
        out += &format!(
            "{},{},{},{},{},{},{:.1}\n",
            p.player.id,
            p.stars,
            p.player.position,
            p.home_state,
            p.cal.signed.map_or(-1, |s| s.program as i64),
            p.cal.offers.len(),
            p.player.overall()
        );
    }
    std::fs::write(format!("{}.recruits.csv", args[3]), out).expect("write");
    eprintln!("{:?}, class of {class}", w.today);
}
