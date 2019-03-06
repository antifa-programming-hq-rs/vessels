use crate::graphics::*;

use stdweb::traits::*;
use stdweb::unstable::TryInto;
use stdweb::web::{
    document, window, CanvasPattern, CanvasRenderingContext2d, FillRule, LineCap, LineJoin,
};

use stdweb::web::event::ResizeEvent;

use stdweb::web::html_element::CanvasElement;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use std::slice::Iter;

use std::ops::Deref;

type CanvasImage = CanvasElement;

impl ImageRepresentation for CanvasImage {}

impl From<Image<RGBA8, Texture2D>> for CanvasImage {
    fn from(input: Image<RGBA8, Texture2D>) -> CanvasImage {
        let canvas: CanvasElement = document()
            .create_element("canvas")
            .unwrap()
            .try_into()
            .unwrap();
        canvas.set_width(input.shape.width);
        canvas.set_height(input.shape.height);
        let context: CanvasRenderingContext2d = canvas.get_context().unwrap();
        let image = context
            .create_image_data(f64::from(input.shape.width), f64::from(input.shape.height))
            .unwrap();
        context.put_image_data(image, 0., 0.).unwrap();
        canvas
    }
}

impl Into<Image<RGBA8, Texture2D>> for CanvasImage {
    fn into(self) -> Image<RGBA8, Texture2D> {
        Image {
            pixels: vec![],
            shape: Texture2D {
                height: 0,
                width: 0,
            },
        }
    }
}

struct CanvasFrame {
    context: CanvasRenderingContext2d,
    canvas: CanvasElement,
    contents: Vec<Object2D<CanvasImage>>,
    pixel_ratio: f64,
    viewport: Cell<Rect2D>,
    size: Cell<Size2D>,
}

impl Drop for CanvasFrame {
    fn drop(&mut self) {
        self.canvas.remove();
    }
}

impl CanvasFrame {
    fn new() -> CanvasFrame {
        let canvas: CanvasElement = document()
            .create_element("canvas")
            .unwrap()
            .try_into()
            .unwrap();
        let context: CanvasRenderingContext2d = canvas.get_context().unwrap();
        CanvasFrame {
            canvas,
            pixel_ratio: window().device_pixel_ratio(),
            context,
            contents: vec![],
            size: Cell::from(Size2D::default()),
            viewport: Cell::from(Rect2D {
                size: Size2D::default(),
                position: Point2D { x: 0., y: 0. },
            }),
        }
    }
    fn show(&self) {
        document().body().unwrap().append_child(&self.canvas);
    }
    fn draw(&self) {
        let viewport = self.viewport.get();
        let size = self.size.get();
        self.context.clear_rect(
            viewport.position.x * self.pixel_ratio,
            viewport.position.y * self.pixel_ratio,
            viewport.size.width * self.pixel_ratio,
            viewport.size.height * self.pixel_ratio,
        );
        self.contents.iter().for_each(|object| {
            let draw = |base_position: Point2D, content: Iter<Path<CanvasImage>>| {
                content.for_each(|entity| {
                    let matrix = entity.orientation.to_matrix();
                    self.context.set_transform(1.,0.,0.,1.,-viewport.position.x * self.pixel_ratio,-viewport.position.y * self.pixel_ratio);
                    self.context.scale(size.width/viewport.size.width, size.height/viewport.size.height);
                    self.context.transform(matrix[0],matrix[1],matrix[2],matrix[3],matrix[4],matrix[5]);
                    self.context.begin_path();
                    match &entity.shadow {
                        Some(shadow) => {
                            self.context.set_shadow_blur(shadow.blur);
                            self.context.set_shadow_color(&shadow.color.to_rgba_color());
                            self.context.set_shadow_offset_x(shadow.offset.x);
                            self.context.set_shadow_offset_y(shadow.offset.y);
                        }
                        None => {
                            self.context.set_shadow_color("rgba(0,0,0,0)");
                        }
                    }
                    let segments = entity.segments.iter();
                    self.context.move_to(entity.orientation.position.x, entity.orientation.position.y);
                    segments.for_each(|segment| {
                        match segment {
                            Segment2D::LineTo(point) => {
                                self.context.line_to(
                                    (base_position.x + point.x + entity.orientation.position.x)
                                        * self.pixel_ratio,
                                    (base_position.x + point.y + entity.orientation.position.y)
                                        * self.pixel_ratio,
                                );
                            },
                            Segment2D::MoveTo(point) => {
                                self.context.move_to(
                                    (base_position.x + point.x + entity.orientation.position.x)
                                        * self.pixel_ratio,
                                    (base_position.x + point.y + entity.orientation.position.y)
                                        * self.pixel_ratio,
                                );
                            },
                            Segment2D::CubicTo(point, handle_1, handle_2) => {
                                self.context.bezier_curve_to(
                                    (base_position.x + handle_1.x + entity.orientation.position.x)
                                        * self.pixel_ratio,
                                    (base_position.x + handle_1.y + entity.orientation.position.y)
                                        * self.pixel_ratio,
                                    (base_position.x + handle_2.x + entity.orientation.position.x)
                                        * self.pixel_ratio,
                                    (base_position.x + handle_2.y + entity.orientation.position.y)
                                        * self.pixel_ratio,
                                    (base_position.x + point.x + entity.orientation.position.x)
                                        * self.pixel_ratio,
                                    (base_position.x + point.y + entity.orientation.position.y)
                                        * self.pixel_ratio,
                                );
                            }
                            Segment2D::QuadraticTo(point, handle) => {
                                self.context.quadratic_curve_to(
                                    (base_position.x + handle.x + entity.orientation.position.x)
                                        * self.pixel_ratio,
                                    (base_position.x + handle.y + entity.orientation.position.y)
                                        * self.pixel_ratio,
                                    (base_position.x + point.x + entity.orientation.position.x)
                                        * self.pixel_ratio,
                                    (base_position.x + point.y + entity.orientation.position.y)
                                        * self.pixel_ratio,
                                );
                            }
                        }
                    });
                    if entity.closed {
                        self.context.close_path();
                    }
                    match &entity.stroke {
                        Some(stroke) => {
                            self.context.set_line_cap(match &stroke.cap {
                                StrokeCapType::Butt => LineCap::Butt,
                                StrokeCapType::Round => LineCap::Round,
                            });
                            self.context.set_line_join(match &stroke.join {
                                StrokeJoinType::Miter => LineJoin::Miter,
                                StrokeJoinType::Round => LineJoin::Round,
                                StrokeJoinType::Bevel => LineJoin::Bevel,
                            });
                            match &stroke.content {
                                VectorTexture::Solid(color) => {
                                    self.context.set_stroke_style_color(&color.to_rgba_color());
                                }
                                VectorTexture::LinearGradient(gradient) => {
                                    let canvas_gradient = self.context.create_linear_gradient(
                                        gradient.start.x,
                                        gradient.start.y,
                                        gradient.end.x,
                                        gradient.end.y,
                                    );
                                    gradient.stops.iter().for_each(|stop| {
                                        canvas_gradient
                                            .add_color_stop(
                                                stop.offset,
                                                &stop.color.to_rgba_color(),
                                            )
                                            .unwrap();
                                    });
                                    self.context.set_stroke_style_gradient(&canvas_gradient);
                                }
                                VectorTexture::Image(image) => {
                                    let pattern: CanvasPattern = js! {
                                        @{&self.context}.createPattern(@{image.deref()}, "no-repeat");
                                    }
                                    .try_into()
                                    .unwrap();
                                    self.context.set_stroke_style_pattern(&pattern);
                                }
                                VectorTexture::RadialGradient(gradient) => {
                                    let canvas_gradient = self
                                        .context
                                        .create_radial_gradient(
                                            gradient.start.x,
                                            gradient.start.y,
                                            gradient.start_radius,
                                            gradient.end.x,
                                            gradient.end.y,
                                            gradient.end_radius,
                                        )
                                        .unwrap();
                                    gradient.stops.iter().for_each(|stop| {
                                        canvas_gradient
                                            .add_color_stop(
                                                stop.offset,
                                                &stop.color.to_rgba_color(),
                                            )
                                            .unwrap();
                                    });
                                    self.context.set_stroke_style_gradient(&canvas_gradient);
                                }
                            }
                            self.context.set_line_width(f64::from(stroke.width) * self.pixel_ratio);
                            self.context.stroke();
                        }
                        None => {}
                    }
                    match &entity.fill {
                        Some(fill) => {
                            match &fill.content {
                                VectorTexture::Solid(color) => {
                                    self.context.set_fill_style_color(&color.to_rgba_color());
                                }
                                VectorTexture::Image(image) => {
                                    let pattern: CanvasPattern = js! {
                                        return @{&self.context}.createPattern(@{image.deref()}, "no-repeat");
                                    }
                                    .try_into()
                                    .unwrap();
                                    self.context.set_fill_style_pattern(&pattern);
                                }
                                VectorTexture::LinearGradient(gradient) => {
                                    let canvas_gradient = self.context.create_linear_gradient(
                                        gradient.start.x,
                                        gradient.start.y,
                                        gradient.end.x,
                                        gradient.end.y,
                                    );
                                    gradient.stops.iter().for_each(|stop| {
                                        canvas_gradient
                                            .add_color_stop(
                                                stop.offset,
                                                &stop.color.to_rgba_color(),
                                            )
                                            .unwrap();
                                    });
                                    self.context.set_fill_style_gradient(&canvas_gradient);
                                }
                                VectorTexture::RadialGradient(gradient) => {
                                    let canvas_gradient = self
                                        .context
                                        .create_radial_gradient(
                                            gradient.start.x,
                                            gradient.start.y,
                                            gradient.start_radius,
                                            gradient.end.x,
                                            gradient.end.y,
                                            gradient.end_radius,
                                        )
                                        .unwrap();
                                    gradient.stops.iter().for_each(|stop| {
                                        canvas_gradient
                                            .add_color_stop(
                                                stop.offset,
                                                &stop.color.to_rgba_color(),
                                            )
                                            .unwrap();
                                    });
                                    self.context.set_fill_style_gradient(&canvas_gradient);
                                }
                            }
                            self.context.fill(FillRule::NonZero);
                        }
                        None => {}
                    }
                });
            };
            let base_position: Point2D;
            let content: Iter<Path<CanvasImage>>;
            match object {
                Object2D::Dynamic(object) => {
                    base_position = object.orientation().position;
                    let _content = object.render();
                    content = _content.iter();
                    draw(base_position, content);
                }
                Object2D::Static(object) => {
                    base_position = object.orientation.position;
                    content = object.content.iter();
                    draw(base_position, content);
                }
            }
        });
    }
}

impl DynamicObject2D<CanvasImage> for CanvasFrame {
    fn orientation(&self) -> Transform2D {
        Transform2D::default()
    }
    fn render(&self) -> Cow<[Path<CanvasImage>]> {
        self.draw();
        let size = self.size.get();
        Cow::from(vec![Path {
            orientation: Transform2D::default(),
            fill: Some(Fill {
                content: VectorTexture::Image(Box::new(self.canvas.clone())),
            }),
            shadow: None,
            stroke: None,
            closed: true,
            segments: vec![
                Segment2D::LineTo(Point2D { x: 0., y: 0. }),
                Segment2D::LineTo(Point2D {
                    x: 0.,
                    y: size.height,
                }),
                Segment2D::LineTo(Point2D {
                    x: size.width,
                    y: size.height,
                }),
                Segment2D::LineTo(Point2D {
                    x: size.width,
                    y: 0.,
                }),
            ],
        }])
    }
}

impl Frame2D<CanvasImage> for CanvasFrame {
    fn add(&mut self, object: Object2D<CanvasImage>) {
        self.contents.push(object);
    }
    fn set_viewport(&self, viewport: Rect2D) {
        self.viewport.set(viewport);
    }
    fn resize(&self, size: Size2D) {
        self.size.set(size);
        self.canvas
            .set_height((size.height * self.pixel_ratio) as u32);
        self.canvas
            .set_width((size.width * self.pixel_ratio) as u32);
    }
    fn get_size(&self) -> Size2D {
        self.size.get()
    }
    fn to_image(&self) -> Box<CanvasImage> {
        self.draw();
        Box::new(self.canvas.clone())
    }
}

struct Canvas {
    state: Rc<RefCell<CanvasState>>,
}

struct CanvasState {
    root_frame: Option<CanvasFrame>,
    size: ObserverCell<Size2D>,
}

impl Graphics2D for Canvas {
    type Image = CanvasImage;
    type Frame = CanvasFrame;
    fn run(self, root: CanvasFrame) {
        let mut state = self.state.borrow_mut();
        root.show();
        state.root_frame = Some(root);
        let cloned = self.clone();
        window().request_animation_frame(move |delta| {
            cloned.animate(delta);
        });
    }
    fn frame(&self) -> CanvasFrame {
        CanvasFrame::new()
    }
}

impl Canvas {
    fn animate(&self, _delta: f64) {
        let state = self.state.borrow_mut();
        match &state.root_frame {
            Some(frame) => {
                if state.size.is_dirty() {
                    let size = state.size.get();
                    frame.resize(size);
                    frame.set_viewport(Rect2D::new(size.width, size.height, 0., 0.));
                }
                frame.draw();
            }
            None => {}
        }
        let cloned = self.clone();
        window().request_animation_frame(move |delta| {
            cloned.animate(delta);
        });
    }
}

impl Clone for Canvas {
    fn clone(&self) -> Canvas {
        Canvas {
            state: self.state.clone(),
        }
    }
}

pub fn new() -> impl Graphics2D {
    document()
        .head()
        .unwrap()
        .append_html(
            r#"
<style>
body, html, canvas {
    height: 100%;
}
body {
    margin: 0;
    overflow: hidden;
}
canvas {
    width: 100%;
}
</style>
            "#,
        )
        .unwrap();

    let body = document().body().unwrap();

    let gfx = Canvas {
        state: Rc::new(RefCell::new(CanvasState {
            size: ObserverCell::new(Size2D {
                width: f64::from(body.offset_width()),
                height: f64::from(body.offset_height()),
            }),
            root_frame: None,
        })),
    };

    let gfx_resize = gfx.clone();

    window().add_event_listener(move |_: ResizeEvent| {
        let state = gfx_resize.state.borrow();
        let body = document().body().unwrap();
        state.size.set(Size2D {
            width: f64::from(body.offset_width()),
            height: f64::from(body.offset_height()),
        });
    });

    gfx
}