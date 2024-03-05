use notan::draw::*;
use notan::math::{vec2, Vec2, Vec3};
use notan::prelude::*;
use rayon::prelude::*;
use static_aabb2d_index::StaticAABB2DIndexBuilder;

const INITIAL_ENTITIES: usize = 30000; //2540;
const INITIAL_VELOCITY: f32 = 40.0;
const ENTITY_RADIUS: f32 = 2.0;
const GAME_WIDTH: f32 = 1280.0;
const GAME_HEIGHT: f32 = 940.0;
const COLLISION_COLOR_TIME: f32 = 0.1;
const ENTITY_COLOR: Color = Color::SILVER;
const ENTITY_COLLISION_COLOR: Color = Color::ORANGE;

struct Body {
    position: Vec2,
    velocity: Vec2,
    force: Vec2,
    radius: f32,
}

struct Transform {
    position: Vec2,
    size: Vec2,
}

struct Entity {
    body: Body,
    transform: Transform,
    is_colliding: bool,
    collision_time: f32,
    follow_mouse: bool,
}

#[derive(AppState)]
struct State {
    entities: Vec<Entity>,
    texture: Texture,
    font: Font,
    pause: bool,
}

#[notan_main]
fn main() -> Result<(), String> {
    let win = WindowConfig::default().set_size(GAME_WIDTH as _, GAME_HEIGHT as _);
    // .set_vsync(true);

    notan::init_with(setup)
        .add_config(win)
        .add_config(DrawConfig)
        .update(update)
        .draw(draw)
        .build()
}

fn setup(gfx: &mut Graphics) -> State {
    let entities = init_entities();
    let texture = gfx
        .create_texture()
        .from_image(include_bytes!("../assets/white_circle.png"))
        .build()
        .unwrap();
    let font = gfx
        .create_font(include_bytes!("../assets/Ubuntu-B.ttf"))
        .unwrap();
    State {
        entities,
        texture,
        font,
        pause: false,
    }
}

fn update(app: &mut App, state: &mut State) {
    if app.keyboard.was_pressed(KeyCode::Space) {
        state.pause = !state.pause;
    }

    if state.pause {
        return;
    }

    // -- logic
    let delta = app.timer.delta_f32();

    sys_clean_collisions(&mut state.entities, delta);

    spawn_big_circle(app, state);
    sys_follow_mouse(&mut state.entities, vec2(app.mouse.x, app.mouse.y));

    sys_apply_movement_to_body(&mut state.entities, delta);
    sys_bounce_rect(&mut state.entities);
    let collisions = sys_check_collision(&mut state.entities);
    sys_resolve_collisions(&mut state.entities, collisions);
    sys_body_to_transform(&mut state.entities);
}

fn draw(app: &mut App, gfx: &mut Graphics, state: &mut State) {
    let mut draw = gfx.create_draw();
    draw.clear(Color::BLACK);

    state.entities.iter().for_each(|e| {
        let pos = e.transform.position - e.transform.size * 0.5;
        let color = if e.collision_time > 0.0 {
            interpolate_color(
                ENTITY_COLOR,
                ENTITY_COLLISION_COLOR,
                COLLISION_COLOR_TIME,
                e.collision_time,
            )
        } else {
            ENTITY_COLOR
        };
        draw.image(&state.texture)
            .position(pos.x, pos.y)
            .size(e.transform.size.x, e.transform.size.y)
            .color(color);
    });

    draw.text(
        &state.font,
        &format!(
            "FPS: {:.2} - MS: {:.3}\nEntities: {}",
            app.timer.fps(),
            app.timer.delta_f32(),
            state.entities.len()
        ),
    )
    .size(30.0)
    .position(10.0, 10.0)
    .v_align_top()
    .h_align_left();

    gfx.render(&draw);
}

fn spawn_big_circle(app: &mut App, state: &mut State) {
    if app.mouse.was_pressed(MouseButton::Left) {
        let position = vec2(GAME_WIDTH * 0.5, GAME_HEIGHT * 0.5);
        let radius = 32.0 + app.timer.elapsed_f32() / 10.0;
        let size = Vec2::splat(radius * 2.0);
        state.entities.push(Entity {
            body: Body {
                position,
                velocity: Default::default(),
                force: Default::default(),
                radius,
            },
            transform: Transform { position, size },
            is_colliding: false,
            collision_time: 0.0,
            follow_mouse: true,
        })
    }
}

fn init_entities() -> Vec<Entity> {
    let mut rng = Random::default();
    (0..INITIAL_ENTITIES)
        .map(|_| {
            let min_pos = vec2(50.0, 50.0);
            let max_pos = vec2(GAME_WIDTH - min_pos.x * 2.0, GAME_HEIGHT - min_pos.y * 2.0);
            let position = vec2(
                min_pos.x + rng.gen::<f32>() * max_pos.x,
                min_pos.y + rng.gen::<f32>() * max_pos.y,
            );
            let min_vel = INITIAL_VELOCITY * -0.5;
            let max_vel = INITIAL_VELOCITY;
            let velocity = vec2(
                min_vel + rng.gen::<f32>() * max_vel,
                min_vel + rng.gen::<f32>() * max_vel,
            );
            Entity {
                body: Body {
                    position,
                    velocity,
                    radius: ENTITY_RADIUS,
                    force: Vec2::splat(0.0),
                },
                transform: Transform {
                    position,
                    size: Vec2::splat(ENTITY_RADIUS * 2.0),
                },
                is_colliding: false,
                collision_time: 0.0,
                follow_mouse: false,
            }
        })
        .collect()
}

fn is_colliding(p1: Vec2, r1: f32, p2: Vec2, r2: f32) -> bool {
    let sum_radius = r1 + r2;
    let square_radius = sum_radius * sum_radius;
    let square_distance = p1.distance_squared(p2);
    square_distance <= square_radius
}

fn interpolate_color(c1: Color, c2: Color, total_time: f32, elapsed: f32) -> Color {
    let c1: Vec3 = c1.rgb().into();
    let c2: Vec3 = c2.rgb().into();
    let delta = c2 - c1;
    let fc = c1 + delta * (elapsed / total_time);
    Color::from_rgb(fc.x, fc.y, fc.z)
}

// systems
fn sys_clean_collisions(entities: &mut [Entity], delta: f32) {
    entities.iter_mut().for_each(|e| {
        e.is_colliding = false;
        if e.collision_time > 0.0 {
            e.collision_time -= delta;
        }
    });
}

fn sys_check_collision(entities: &mut [Entity]) -> Vec<(usize, Vec<usize>)> {
    let mut builder = StaticAABB2DIndexBuilder::new(entities.len());
    entities.iter().for_each(|e1| {
        let p = e1.body.position;
        let r = e1.body.radius;
        let min = p - r;
        let max = p + r;
        builder.add(min.x, min.y, max.x, max.y);
    });

    let collisions = builder.build().unwrap();

    entities
        .par_iter()
        .enumerate()
        .map(|(id1, e)| {
            let p1 = e.body.position;
            let r1 = e.body.radius;
            let min = p1 - r1;
            let max = p1 + r1;
            let cols = collisions.query(min.x, min.y, max.x, max.y);
            let mut colliding_with = vec![];
            for id2 in cols {
                if id1 == id2 {
                    continue;
                }

                let e2 = &entities[id2];
                let p2 = e2.body.position;
                let r2 = e2.body.radius;

                if !is_colliding(p1, r1, p2, r2) {
                    continue;
                }

                colliding_with.push(id2);
            }

            (id1, colliding_with)
        })
        .collect::<Vec<_>>()
}

fn sys_resolve_collisions(entities: &mut [Entity], collisions: Vec<(usize, Vec<usize>)>) {
    collisions.into_iter().for_each(|(id1, cols)| {
        let e1 = &entities[id1];
        let p1 = e1.body.position;
        let r1 = e1.body.radius;

        cols.into_iter().for_each(|id2| {
            let e2 = &entities[id2];
            let p2 = e2.body.position;
            let r2 = e2.body.radius;

            let pos_delta = p1 - p2;
            let sum_radius = r1 + r2;
            let distance = pos_delta.length();
            let penetration = sum_radius - distance;

            let direction = (p2 - p1).normalize_or_zero();

            // Move the circles away from each other by half the penetration depth
            let e1 = &mut entities[id1];
            e1.is_colliding = true;
            e1.collision_time = COLLISION_COLOR_TIME;

            if r1 < r2 {
                let push_force = penetration * (r2 / (r1 + r2));
                e1.body.position -= direction * push_force;
            }

            let e2 = &mut entities[id2];
            e2.is_colliding = true;
            e2.collision_time = COLLISION_COLOR_TIME;

            if r1 >= r2 {
                let push_force = penetration * (r1 / (r1 + r2));
                e2.body.position += direction * push_force;
            }
        });
    });
}

fn sys_bounce_rect(entities: &mut [Entity]) {
    entities.iter_mut().for_each(|e| {
        let left = e.body.position.x - e.body.radius <= 0.0;
        if left {
            e.body.velocity.x *= -1.0;
            e.body.position.x = e.body.radius;
        }
        let right = e.body.position.x + e.body.radius >= GAME_WIDTH;
        if right {
            e.body.velocity.x *= -1.0;
            e.body.position.x = GAME_WIDTH - e.body.radius;
        }
        let top = e.body.position.y - e.body.radius < 0.0;
        if top {
            e.body.velocity.y *= -1.0;
            e.body.position.y = e.body.radius;
        }
        let bottom = e.body.position.y + e.body.radius >= GAME_HEIGHT;
        if bottom {
            e.body.velocity.y *= -1.0;
            e.body.position.y = GAME_HEIGHT - e.body.radius;
        }
    });
}

fn sys_apply_movement_to_body(entities: &mut [Entity], delta: f32) {
    entities.iter_mut().for_each(|e| {
        let vel = e.body.velocity + e.body.force;
        e.body.position += vel * delta;
        e.body.force = Vec2::ZERO;
    });
}

fn sys_body_to_transform(entites: &mut [Entity]) {
    entites.iter_mut().for_each(|e| {
        e.transform.position = e.body.position;
    });
}

fn sys_follow_mouse(entities: &mut [Entity], pos: Vec2) {
    entities.iter_mut().for_each(|e| {
        if !e.follow_mouse {
            return;
        }
        let normalized_direction = (pos - e.body.position).normalize_or_zero();
        e.body.force += 90.0 * normalized_direction;
    });
}
