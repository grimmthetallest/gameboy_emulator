#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate stdweb_derive;
extern crate gameboy_core;

mod screen;
mod webgl_rendering_context;

use gameboy_core::{Button, Cartridge, Controller, ControllerEvent, Emulator};
use screen::Screen;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::mpsc::TryRecvError;
use stdweb::traits::*;
use stdweb::unstable::TryInto;
use stdweb::web::event::{
    ConcreteEvent, KeyDownEvent, KeyUpEvent, MouseDownEvent, MouseUpEvent, TouchEnd, TouchStart,
};
use stdweb::web::html_element::CanvasElement;
use stdweb::web::{document, window, Element, IEventTarget, TypedArray};
use webgl_rendering_context::*;

type Gl = WebGLRenderingContext;

pub fn start(rom: Vec<u8>) {
    let (sender, receiver) = mpsc::channel();
    let run = Rc::new(RefCell::new(true));
    let should_save_to_local = Rc::new(RefCell::new(false));

    let up_btn = document().get_element_by_id("up-btn").unwrap();
    let down_btn = document().get_element_by_id("down-btn").unwrap();
    let left_btn = document().get_element_by_id("left-btn").unwrap();
    let right_btn = document().get_element_by_id("right-btn").unwrap();
    let up_left_btn = document().get_element_by_id("up-left-btn").unwrap();
    let up_right_btn = document().get_element_by_id("up-right-btn").unwrap();
    let down_left_btn = document().get_element_by_id("down-left-btn").unwrap();
    let down_right_btn = document().get_element_by_id("down-right-btn").unwrap();
    let a_btn = document().get_element_by_id("a-btn").unwrap();
    let b_btn = document().get_element_by_id("b-btn").unwrap();
    let start_btn = document().get_element_by_id("start-btn").unwrap();
    let select_btn = document().get_element_by_id("select-btn").unwrap();

    add_button_event_listeners(&up_btn, Button::Up, sender.clone());
    add_button_event_listeners(&down_btn, Button::Down, sender.clone());
    add_button_event_listeners(&left_btn, Button::Left, sender.clone());
    add_button_event_listeners(&right_btn, Button::Right, sender.clone());
    add_button_event_listeners(&start_btn, Button::Start, sender.clone());
    add_button_event_listeners(&select_btn, Button::Select, sender.clone());
    add_button_event_listeners(&a_btn, Button::A, sender.clone());
    add_button_event_listeners(&b_btn, Button::B, sender.clone());

    add_multi_button_event_listeners(&up_left_btn, Button::Up, Button::Left, sender.clone());
    add_multi_button_event_listeners(&up_right_btn, Button::Up, Button::Right, sender.clone());
    add_multi_button_event_listeners(&down_left_btn, Button::Down, Button::Left, sender.clone());
    add_multi_button_event_listeners(&down_right_btn, Button::Down, Button::Right, sender.clone());

    {
        let sender = sender.clone();
        window().add_event_listener(move |event: KeyDownEvent| {
            let _send_result = match event.key().as_ref() {
                "ArrowUp" => Some(sender.send(ControllerEvent::Pressed(Button::Up))),
                "ArrowDown" => Some(sender.send(ControllerEvent::Pressed(Button::Down))),
                "ArrowLeft" => Some(sender.send(ControllerEvent::Pressed(Button::Left))),
                "ArrowRight" => Some(sender.send(ControllerEvent::Pressed(Button::Right))),
                "z" => Some(sender.send(ControllerEvent::Pressed(Button::A))),
                "x" => Some(sender.send(ControllerEvent::Pressed(Button::B))),
                "Enter" => Some(sender.send(ControllerEvent::Pressed(Button::Select))),
                " " => Some(sender.send(ControllerEvent::Pressed(Button::Start))),
                _ => None,
            };
        });
    }

    {
        let sender = sender.clone();
        window().add_event_listener(move |event: KeyUpEvent| {
            let _send_result = match event.key().as_ref() {
                "ArrowUp" => Some(sender.send(ControllerEvent::Released(Button::Up))),
                "ArrowDown" => Some(sender.send(ControllerEvent::Released(Button::Down))),
                "ArrowLeft" => Some(sender.send(ControllerEvent::Released(Button::Left))),
                "ArrowRight" => Some(sender.send(ControllerEvent::Released(Button::Right))),
                "z" => Some(sender.send(ControllerEvent::Released(Button::A))),
                "x" => Some(sender.send(ControllerEvent::Released(Button::B))),
                "Enter" => Some(sender.send(ControllerEvent::Released(Button::Select))),
                " " => Some(sender.send(ControllerEvent::Released(Button::Start))),
                _ => None,
            };
        });
    }

    let canvas: CanvasElement = document()
        .get_element_by_id("canvas")
        .unwrap()
        .try_into()
        .unwrap();

    // TODO: move this to another function
    let gl: Gl = canvas.get_context().unwrap();

    gl.clear_color(1.0, 0.0, 0.0, 1.0);
    gl.clear(Gl::COLOR_BUFFER_BIT);

    let verticies: [f32; 12] = [
        1.0, 1.0, 0.0, 1.0, -1.0, 0.0, -1.0, -1.0, 0.0, -1.0, 1.0, 0.0,
    ];
    let vertex_array = TypedArray::<f32>::from(verticies.as_ref()).buffer();
    let vertex_buffer = gl.create_buffer().unwrap();
    gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&vertex_buffer));
    gl.buffer_data_1(Gl::ARRAY_BUFFER, Some(&vertex_array), Gl::STATIC_DRAW);

    let texture_coordinate: [f32; 8] = [1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];
    let texture_array = TypedArray::<f32>::from(texture_coordinate.as_ref()).buffer();
    let texture_buffer = gl.create_buffer().unwrap();
    gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&texture_buffer));
    gl.buffer_data_1(Gl::ARRAY_BUFFER, Some(&texture_array), Gl::STATIC_DRAW);

    let indicies: [u8; 6] = [0, 1, 3, 1, 2, 3];
    let indicies_array = TypedArray::<u8>::from(indicies.as_ref()).buffer();
    let index_buffer = gl.create_buffer().unwrap();
    gl.bind_buffer(Gl::ELEMENT_ARRAY_BUFFER, Some(&index_buffer));
    gl.buffer_data_1(
        Gl::ELEMENT_ARRAY_BUFFER,
        Some(&indicies_array),
        Gl::STATIC_DRAW,
    );

    let vertex_source: &str = include_str!("shaders/vertex.glsl");
    let vert_shader = match compile_shader(&gl, Gl::VERTEX_SHADER, vertex_source) {
        Ok(shader) => shader,
        Err(msg) => panic!(msg),
    };

    let fragment_source: &str = include_str!("shaders/fragment.glsl");
    let frag_shader = match compile_shader(&gl, Gl::FRAGMENT_SHADER, fragment_source) {
        Ok(shader) => shader,
        Err(msg) => panic!(msg),
    };

    let shader_program = link_program(&gl, &vert_shader, &frag_shader);

    gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&vertex_buffer));
    let pos_attr = gl.get_attrib_location(&shader_program, "aPos") as u32;
    gl.vertex_attrib_pointer(pos_attr, 3, Gl::FLOAT, false, 0, 0);
    gl.enable_vertex_attrib_array(pos_attr);

    gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&texture_buffer));
    let tex_attr = gl.get_attrib_location(&shader_program, "aTexCoord") as u32;
    gl.vertex_attrib_pointer(tex_attr, 2, Gl::FLOAT, false, 0, 0);
    gl.enable_vertex_attrib_array(tex_attr);

    let texture = gl.create_texture().unwrap();
    gl.bind_texture(Gl::TEXTURE_2D, Some(&texture));

    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MIN_FILTER, Gl::NEAREST as i32);
    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MAG_FILTER, Gl::NEAREST as i32);
    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_S, Gl::CLAMP_TO_EDGE as i32);
    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_T, Gl::CLAMP_TO_EDGE as i32);

    let mut cartridge = Cartridge::from_rom(rom);

    if let Some(ram_str) = window().local_storage().get(cartridge.get_name()) {
        let chars: Vec<char> = ram_str.chars().collect();
        let bytes: Vec<u8> = chars
            .chunks(2)
            .map(|chunk| {
                let byte: String = chunk.iter().collect();
                u8::from_str_radix(&byte, 16).unwrap()
            })
            .collect();
        cartridge.set_ram(bytes);
    }

    let mut emulator = Emulator::from_cartridge(cartridge);
    let screen = Screen::new();
    let controller = Controller::new();

    let has_battery = emulator.get_cartridge().has_battery();
    let ram = emulator.get_cartridge().get_ram().to_vec();
    let ram_str: Rc<RefCell<String>> = Rc::new(RefCell::new(
        ram.iter().map(|byte| format!("{:02x}", byte)).collect(),
    ));
    {
        let should_save_to_local = should_save_to_local.clone();
        let ram_str = ram_str.clone();
        emulator.set_ram_change_callback(Box::new(move |address, value| {
            if has_battery {
                let byte_chars: Vec<char> = format!("{:02x}", value).chars().collect();
                let (first, second) = (byte_chars[0] as u8, byte_chars[1] as u8);
                unsafe {
                    let mut ram_str_ref = ram_str.borrow_mut();
                    let bytes = ram_str_ref.as_bytes_mut();
                    bytes[address * 2] = first;
                    bytes[address * 2 + 1] = second;
                }
                *should_save_to_local.borrow_mut() = true;
            }
        }));
    }

    main_loop(
        emulator,
        screen,
        controller,
        receiver,
        run.clone(),
        should_save_to_local.clone(),
        ram_str.clone(),
        gl,
        shader_program,
        texture,
    );
}

fn add_button_event_listeners(
    element: &Element,
    button: Button,
    sender: mpsc::Sender<ControllerEvent>,
) {
    add_controller_event_listener::<MouseDownEvent>(
        element,
        ControllerEvent::Pressed(button),
        sender.clone(),
    );
    add_controller_event_listener::<MouseUpEvent>(
        element,
        ControllerEvent::Released(button),
        sender.clone(),
    );
    add_controller_event_listener::<TouchStart>(
        element,
        ControllerEvent::Pressed(button),
        sender.clone(),
    );
    add_controller_event_listener::<TouchEnd>(
        element,
        ControllerEvent::Released(button),
        sender.clone(),
    );
}

fn add_controller_event_listener<T: ConcreteEvent>(
    element: &Element,
    controller_event: ControllerEvent,
    sender: mpsc::Sender<ControllerEvent>,
) {
    element.add_event_listener(move |_: T| {
        sender.send(controller_event).unwrap();
    });
}

fn add_multi_button_event_listeners(
    element: &Element,
    first_button: Button,
    second_button: Button,
    sender: mpsc::Sender<ControllerEvent>,
) {
    add_multi_controller_event_listener::<MouseDownEvent>(
        element,
        ControllerEvent::Pressed(first_button),
        ControllerEvent::Pressed(second_button),
        sender.clone(),
    );
    add_multi_controller_event_listener::<MouseUpEvent>(
        element,
        ControllerEvent::Released(first_button),
        ControllerEvent::Released(second_button),
        sender.clone(),
    );
    add_multi_controller_event_listener::<TouchStart>(
        element,
        ControllerEvent::Pressed(first_button),
        ControllerEvent::Pressed(second_button),
        sender.clone(),
    );
    add_multi_controller_event_listener::<TouchEnd>(
        element,
        ControllerEvent::Released(first_button),
        ControllerEvent::Released(second_button),
        sender.clone(),
    );
}

fn add_multi_controller_event_listener<T: ConcreteEvent>(
    element: &Element,
    first_controller_event: ControllerEvent,
    second_controller_event: ControllerEvent,
    sender: mpsc::Sender<ControllerEvent>,
) {
    element.add_event_listener(move |_: T| {
        sender.send(first_controller_event).unwrap();
        sender.send(second_controller_event).unwrap();
    });
}

fn compile_shader(gl: &Gl, shader_type: GLenum, source: &str) -> Result<WebGLShader, String> {
    let shader = gl.create_shader(shader_type).unwrap();
    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);
    let compiled = gl.get_shader_parameter(&shader, Gl::COMPILE_STATUS);

    if compiled == stdweb::Value::Bool(false) {
        let error = gl.get_shader_info_log(&shader);
        Err(error.unwrap_or_else(|| "Unknown compilation error".to_string()))
    } else {
        Ok(shader)
    }
}

fn link_program(gl: &Gl, vert_shader: &WebGLShader, frag_shader: &WebGLShader) -> WebGLProgram {
    let shader_program = gl.create_program().unwrap();
    gl.attach_shader(&shader_program, vert_shader);
    gl.attach_shader(&shader_program, frag_shader);
    gl.link_program(&shader_program);
    shader_program
}

// since closure's in Rust can't be recursive, this function has too many args
#[allow(clippy::too_many_arguments)]
fn main_loop(
    mut emulator: Emulator,
    mut screen: Screen,
    mut controller: Controller,
    receiver: mpsc::Receiver<ControllerEvent>,
    run: Rc<RefCell<bool>>,
    should_save_to_local: Rc<RefCell<bool>>,
    ram_str: Rc<RefCell<String>>,
    gl: Gl,
    shader_program: WebGLProgram,
    texture: WebGLTexture,
) {
    loop {
        let vblank = emulator.emulate(&mut screen, &mut controller);
        if vblank {
            break;
        }
    }

    loop {
        match receiver.try_recv() {
            Ok(ControllerEvent::Pressed(button)) => controller.press(button),
            Ok(ControllerEvent::Released(button)) => controller.release(button),
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                *run.borrow_mut() = false;
                break;
            }
        }
    }

    if *should_save_to_local.borrow() {
        let name = emulator.get_cartridge().get_name();
        window()
            .local_storage()
            .insert(&name, &ram_str.borrow())
            .unwrap();
        *should_save_to_local.borrow_mut() = false;
    }

    let frame_buffer = screen.get_frame_buffer();
    render(&gl, &shader_program, &texture, frame_buffer.as_ref());

    if *run.borrow() {
        window().request_animation_frame(move |_| {
            main_loop(
                emulator,
                screen,
                controller,
                receiver,
                run,
                should_save_to_local,
                ram_str,
                gl,
                shader_program,
                texture,
            );
        });
    }
}

fn render(gl: &Gl, shader_program: &WebGLProgram, texture: &WebGLTexture, frame_buffer: &[u8]) {
    gl.bind_texture(Gl::TEXTURE_2D, Some(texture));
    gl.tex_image2_d(
        Gl::TEXTURE_2D,
        0,
        Gl::RGB as i32,
        160,
        144,
        0,
        Gl::RGB,
        Gl::UNSIGNED_BYTE,
        Some(frame_buffer),
    );
    gl.active_texture(Gl::TEXTURE0);
    gl.use_program(Some(shader_program));
    let screen_uniform = gl.get_uniform_location(&shader_program, "screen").unwrap();
    gl.uniform1i(Some(&screen_uniform), 0);
    gl.draw_elements(Gl::TRIANGLES, 6, Gl::UNSIGNED_BYTE, 0);
}
