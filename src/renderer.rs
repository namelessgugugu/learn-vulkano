use std::sync::Arc;
use std::fs::File;
use std::io::Read;

use ahash::HashSet;

use vulkano::device::Device;
use vulkano::pipeline::layout::{PipelineLayout, PipelineLayoutCreateInfo};
use vulkano::format::{Format, ClearValue};
use vulkano::render_pass::RenderPass;
use vulkano::render_pass::Subpass;
use vulkano::shader::{ShaderModule, ShaderModuleCreateInfo};
use vulkano::pipeline::{PipelineCreateFlags, PipelineShaderStageCreateInfo};
use vulkano::pipeline::graphics::{GraphicsPipeline, GraphicsPipelineCreateInfo};
use vulkano::pipeline::graphics::vertex_input::VertexInputState;
use vulkano::pipeline::graphics::input_assembly::{InputAssemblyState, PrimitiveTopology};
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::color_blend::{ColorBlendState, ColorBlendAttachmentState};
use vulkano::pipeline::DynamicState;
use vulkano::pipeline::graphics::subpass::PipelineSubpassType;
use vulkano::image::view::ImageView;
use vulkano::command_buffer::auto::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer};
use vulkano::command_buffer::{
    CommandBufferUsage, RenderPassBeginInfo, SubpassBeginInfo, SubpassEndInfo,
};
use vulkano::image::ImageLayout;
use vulkano::render_pass::{
    Framebuffer, FramebufferCreateInfo, RenderPassCreateInfo, AttachmentDescription,
    AttachmentLoadOp, AttachmentStoreOp, AttachmentReference, SubpassDescription
};

use smallvec::SmallVec;

use crate::resources::Resources;
#[derive(Debug)]
pub struct Renderer {
    pub pipeline_layout: Arc<PipelineLayout>,
    pub render_pass: Arc<RenderPass>,
    pub graphics_pipeline: Arc<GraphicsPipeline>
}

impl Renderer {
    fn new_pipeline_layout(device: Arc<Device>) -> Arc<PipelineLayout> {
        let create_info = PipelineLayoutCreateInfo::default();
        PipelineLayout::new(device, create_info).expect("Fail to create pipeline layout.")
    }
    fn new_render_pass(device: Arc<Device>, format: Format) -> Arc<RenderPass> {
        let color_attachment = AttachmentDescription {
            format: format,
            load_op: AttachmentLoadOp::Clear,
            store_op: AttachmentStoreOp::Store,
            initial_layout: ImageLayout::Undefined,
            final_layout: ImageLayout::PresentSrc,
            ..Default::default()
        };
        let attachments = vec![color_attachment];
        let color_attachment_ref = AttachmentReference {
            attachment: 0,
            layout: ImageLayout::ColorAttachmentOptimal,
            ..Default::default()
        };
        let color_attachments = vec![Some(color_attachment_ref)];
        let subpass_description = SubpassDescription {
            color_attachments,
            ..Default::default()
        };
        let subpasses = vec![subpass_description];
        let create_info = RenderPassCreateInfo {
            attachments,
            subpasses,
            ..Default::default()
        };
        RenderPass::new(device, create_info)
            .expect("Fail to create render pass")
    }
    fn read_spirv_code(device: Arc<Device>, path: String) -> Arc<ShaderModule> {
        let mut handler = File::open(path).expect("Fail to open the spv file.");
        let mut bytes = Vec::new();
        handler.read_to_end(&mut bytes).expect("Fail to read the spv file.");
        let words = vulkano::shader::spirv::bytes_to_words(bytes.as_slice())
            .expect("Fail to translate spir-v bytes to words.");
        let create_info = ShaderModuleCreateInfo::new(&*words);
        unsafe { ShaderModule::new(device, create_info).expect("Fail to create shader module.") }
    }
    fn new_graphics_pipeline(
        device: Arc<Device>,
        pipeline_layout: Arc<PipelineLayout>,
        subpass: Subpass
    ) -> Arc<GraphicsPipeline> {
        let flags = PipelineCreateFlags::empty();
        
        let vertex_shader = Self::read_spirv_code(device.clone(), String::from(".\\shaders\\vert.spv"));
        let fragment_shader = Self::read_spirv_code(device.clone(), String::from(".\\shaders\\frag.spv"));

        let stages = {
            let vertex_shader_stage = PipelineShaderStageCreateInfo::new(
                vertex_shader.entry_point("main").expect("Fail to find entry point"
            ));
            let fragment_shader_stage = PipelineShaderStageCreateInfo::new(
                fragment_shader.entry_point("main").expect("Fail to find entry point"
            ));
            SmallVec::from_vec(vec![vertex_shader_stage, fragment_shader_stage])
        };

        let vertex_input_state = Some(
            VertexInputState::new()
        );

        let input_assembly_state = Some(
            InputAssemblyState {
                topology: PrimitiveTopology::TriangleList,
                ..Default::default()
            }
        );

        let tessellation_state = None;

        let viewport_state = Some(
            ViewportState::default()
        );

        let rasterization_state = Some(
            RasterizationState::default()
        );

        let multisample_state = Some(
            MultisampleState::default()
        );

        let depth_stencil_state = None;

        let color_blend_state = Some(
            ColorBlendState {
                attachments: vec![
                    ColorBlendAttachmentState::default()
                ],
                ..Default::default()
            }
        );

        let dynamic_state = {
            let mut incomplete_set = HashSet::default();
            incomplete_set.insert(DynamicState::Viewport);
            incomplete_set
        };

        let subpass = Some(
            PipelineSubpassType::BeginRenderPass(subpass)
        );

        let create_info = GraphicsPipelineCreateInfo {
            flags,
            stages,
            vertex_input_state,
            input_assembly_state,
            tessellation_state,
            viewport_state,
            rasterization_state,
            multisample_state,
            depth_stencil_state,
            color_blend_state,
            dynamic_state,
            subpass,
            ..GraphicsPipelineCreateInfo::layout(pipeline_layout.clone())
        };

        GraphicsPipeline::new(device, None, create_info)
            .expect("Fail to create graphics pipeline.")
    }
    pub fn new(device: Arc<Device>, format: Format) -> Self {
        let pipeline_layout = Self::new_pipeline_layout(device.clone());

        let render_pass = Self::new_render_pass(device.clone(), format);

        let subpass = Subpass::from(render_pass.clone(), 0).unwrap();

        let graphics_pipeline = Self::new_graphics_pipeline(device.clone(), pipeline_layout.clone(), subpass);

        Renderer {
            pipeline_layout,
            render_pass,
            graphics_pipeline
        }
    }
    pub fn record_command_buffer(
        &self,
        resources: &Resources,
        output: Arc<ImageView>,
        graphics_queue_family_index: u32,
    ) -> Arc<PrimaryAutoCommandBuffer> {
        let (render_area_extent, layers) = {
            let extent = output.image().extent();
            ([extent[0], extent[1]], extent[2])
        };
        let framebuffer = {
            let create_info = FramebufferCreateInfo {
                attachments: vec![output.clone()],
                layers,
                ..Default::default()
            };
            Framebuffer::new(self.render_pass.clone(), create_info)
                .expect("Fail to create framebuffer.")
        };
        let clear_values = vec![
            Some(
                ClearValue::Float([1.0, 0.0, 0.0, 0.0])
            )
        ];
        let render_pass_begin_info = RenderPassBeginInfo {
            render_area_extent,
            clear_values,
            ..RenderPassBeginInfo::framebuffer(framebuffer)
        };
        let subpass_begin_info = SubpassBeginInfo::default();
        let subpass_end_info = SubpassEndInfo::default();
        let viewports: SmallVec<[Viewport; 2]> = SmallVec::from_vec(
            vec![
                Viewport {
                    extent: [render_area_extent[0] as f32, render_area_extent[1] as f32],
                    ..Default::default()
                }
            ]
        );

        let mut builder = AutoCommandBufferBuilder::primary(
            &resources.command_buffer_allocator,
            graphics_queue_family_index,
            CommandBufferUsage::OneTimeSubmit
        ).expect("Fail to create command buffer builder.");
        builder.begin_render_pass(render_pass_begin_info, subpass_begin_info).expect("Fail to begin rendering.")
        .bind_pipeline_graphics(self.graphics_pipeline.clone()).expect("Fail to bind graphics pipeline.")
        .set_viewport(0, viewports).expect("Fail to set viewport.")
        .draw(3, 1, 0, 0).expect("Fail to draw vertices.")
        .end_render_pass(subpass_end_info).expect("Fail to end rendering.");
        builder.build().expect("Fail to build command buffer.")
    }
}