use notan::draw::*;
use notan::math::{vec2, Vec2};
use notan::prelude::*;

struct Body {
    position: Vec2,
    velocity: Vec2,
}

struct Transform {
    position: Vec2,
    size: Vec2,
}

struct Entity {
    body: Body,
    transform: Transform,
    collider: f32, // radius
}

#[derive(AppState)]
struct State {
    rng: Random,
    entities: Vec<Entity>,
}

#[notan_main]
fn main() -> Result<(), String> {
    notan::init_with(setup)
        .add_config(DrawConfig)
        .update(update)
        .draw(draw)
        .build()
}

fn setup() -> State {
    let mut rng = Random::default();
    let entities = (0..200)
        .map(|_| {
            let position = vec2(rng.gen::<f32>() * 800.0, rng.gen::<f32>() * 600.0);
            Entity {
                body: Body {
                    position,
                    velocity: Default::default(),
                },
                transform: Transform {
                    position,
                    size: vec2(32.0, 32.0),
                },
                collider: 16.0,
            }
        })
        .collect::<Vec<_>>();
    State { rng, entities }
}

fn update(app: &mut App) {}

fn draw(app: &mut App, gfx: &mut Graphics, state: &mut State) {
    let mut draw = gfx.create_draw();
    draw.clear(Color::BLACK);

    state.entities.iter().for_each(|e| {
        let pos = e.transform.position - e.transform.size * 0.5;
        draw.rect(pos.into(), e.transform.size.into())
            .color(Color::WHITE)
            .alpha(0.5)
            .stroke(2.0);

        draw.circle(e.collider)
            .position(e.transform.position.x, e.transform.position.y)
            .color(Color::GREEN)
            .stroke(1.0);
    });

    gfx.render(&draw);
}
