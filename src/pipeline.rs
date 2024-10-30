pub use vk::{PrimitiveTopology, PolygonMode, DescriptorType, ShaderStageFlags};

use std::ffi::CString;
use ash::vk::{self, VertexInputAttributeDescription, VertexInputBindingDescription};
use crate::{descriptor::Descriptor, Vust};

pub struct GraphicsPipeline {
    descriptor_pool_create_info: Option<(vk::DescriptorPoolCreateInfo, Vec<vk::DescriptorPoolSize>)>,
    descriptor_set_layout: vk::DescriptorSetLayout,
    write_descriptor_set_info: Vec<[vk::WriteDescriptorSet; Vust::MAX_FRAMES_IN_FLIGHT]>,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline
}

impl GraphicsPipeline {
    pub fn new(vust: &Vust, create_info: GraphicsPipelineCreateInfo) -> Self {
        unsafe {
            let vertex_input_state = vust.device.create_shader_module(&vk::ShaderModuleCreateInfo {
                s_type: vk::StructureType::SHADER_MODULE_CREATE_INFO,
                code_size: create_info.vertex_bin.len(),
                p_code: create_info.vertex_bin.as_ptr() as *const u32,
                ..Default::default()
            }, None).unwrap();

            let fragment_input_state = vust.device.create_shader_module(&vk::ShaderModuleCreateInfo {
                s_type: vk::StructureType::SHADER_MODULE_CREATE_INFO,
                code_size: create_info.fragment_bin.len(),
                p_code: create_info.fragment_bin.as_ptr() as *const u32,
                ..Default::default()
            }, None).unwrap();

            let entry_point_name = CString::new("main").unwrap();

            let shader_stages = [
                vk::PipelineShaderStageCreateInfo::builder()
                    .stage(vk::ShaderStageFlags::VERTEX)
                    .module(vertex_input_state)
                    .name(&entry_point_name)
                    .build(),
                vk::PipelineShaderStageCreateInfo::builder()
                    .stage(vk::ShaderStageFlags::FRAGMENT)
                    .module(fragment_input_state)
                    .name(&entry_point_name)
                    .build()
            ];

            let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
                .vertex_binding_descriptions(&create_info.vertex_binding_descriptions)
                .vertex_attribute_descriptions(&create_info.vertex_attribute_descriptions)
                .build();

            let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
                .topology(create_info.topology)
                .primitive_restart_enable(false)
                .build();

            // doing this cuz release build deletes &[viewport] out of existence
            let mut hold_viewport = None;
            let mut hold_scissor = None;
            // absolute unit of a match statement
            let viewport_state_info = match (create_info.viewport.clone(), create_info.scissor.clone()) {
                (Viewport::Dynamic, Scissor::Dynamic) => {
                    vk::PipelineViewportStateCreateInfo::builder().viewport_count(1).scissor_count(1).build()
                }
                (Viewport::Dynamic, Scissor::Static { x, y, width, height }) => {
                    hold_scissor = Some([
                        vk::Rect2D {
                            offset: vk::Offset2D { x, y },
                            extent: vk::Extent2D { width, height }
                        }
                    ]);
                    
                    vk::PipelineViewportStateCreateInfo::builder()
                        .viewport_count(1)
                        .scissors(hold_scissor.as_ref().unwrap())
                        .build()
                }
                (Viewport::Static { x, y, width, height, min_depth, max_depth }, Scissor::Dynamic) => {
                    hold_viewport = Some([
                        vk::Viewport {
                            x,
                            y,
                            width,
                            height,
                            min_depth,
                            max_depth
                        }
                    ]);
                    vk::PipelineViewportStateCreateInfo::builder()
                        .viewports(hold_viewport.as_ref().unwrap())
                        .scissor_count(1)
                        .build()
                }
                (Viewport::Static { x: v_x, y: v_y, width: v_width, height: v_height, min_depth: v_min_depth, max_depth: v_max_depth }, Scissor::Static { x: s_x, y: s_y, width: s_width, height: s_height }) => {
                    hold_viewport = Some([
                        vk::Viewport {
                            x: v_x,
                            y: v_y,
                            width: v_width,
                            height: v_height,
                            min_depth: v_min_depth,
                            max_depth: v_max_depth
                        }
                    ]);
                    hold_scissor = Some([
                        vk::Rect2D {
                            offset: vk::Offset2D { x: s_x, y: s_y },
                            extent: vk::Extent2D { width: s_width, height: s_height }
                        }
                    ]);
                    
                    vk::PipelineViewportStateCreateInfo::builder()
                        .viewports(hold_viewport.as_ref().unwrap())
                        .scissors(hold_scissor.as_ref().unwrap())
                        .build()
                }
            };

            let rasterizer_info = vk::PipelineRasterizationStateCreateInfo::builder()
                .depth_clamp_enable(false)
                .rasterizer_discard_enable(false)
                .polygon_mode(create_info.polygon_mode)
                .line_width(1.0)
                .cull_mode(if let CullMode::None = create_info.cull_mode { vk::CullModeFlags::NONE } else { vk::CullModeFlags::BACK })
                .front_face(
                    match &create_info.cull_mode {
                        CullMode::None => vk::FrontFace::CLOCKWISE, // doesnt matter cuz cull mode is none
                        CullMode::Clockwise => vk::FrontFace::CLOCKWISE,
                        CullMode::AntiClockwise => vk::FrontFace::COUNTER_CLOCKWISE
                    }
                )
                .depth_bias_enable(false)
                .build();

            let multisample_info = vk::PipelineMultisampleStateCreateInfo::builder()
                .rasterization_samples(vk::SampleCountFlags::TYPE_1)
                .sample_shading_enable(false)
                .build();

            let color_blend_attachment = vk::PipelineColorBlendAttachmentState::builder()
                .color_write_mask(vk::ColorComponentFlags::RGBA)
                .blend_enable(true)
                .color_blend_op(vk::BlendOp::ADD)
                .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .src_alpha_blend_factor(vk::BlendFactor::ONE)
                .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
                .alpha_blend_op(vk::BlendOp::ADD)
                .build();
            
            let attachements = [color_blend_attachment];

            let color_blend_info = vk::PipelineColorBlendStateCreateInfo::builder()
                .attachments(&attachements)
                .logic_op_enable(true)
                .logic_op(vk::LogicOp::COPY)
                .blend_constants([0.0, 0.0, 0.0, 0.0])
                .build();

            let bindings = if let Some(descriptor_set_layout) = &create_info.descriptor_set_layout {
                descriptor_set_layout.bindings.iter().enumerate().map(|(i, descriptor_set_binding)| {
                    vk::DescriptorSetLayoutBinding::builder()
                        .binding(i as u32)
                        .descriptor_type(descriptor_set_binding.descriptor_type)
                        .descriptor_count(1)
                        .stage_flags(descriptor_set_binding.stage_flags)
                        .build()
                }).collect::<Vec<_>>()
            } else {
                Vec::new()
            };

            let descriptor_set_layout = vust.device.create_descriptor_set_layout(&vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings).build(), None).unwrap();
            let descriptor_set_layouts = [descriptor_set_layout];
            let pipeline_layout = vust.device.create_pipeline_layout(&vk::PipelineLayoutCreateInfo::builder().set_layouts(&descriptor_set_layouts).build(), None).unwrap();

            let mut dynamic_states = Vec::new();

            if let Viewport::Dynamic = create_info.viewport {
                dynamic_states.push(vk::DynamicState::VIEWPORT);
            }
            if let Scissor::Dynamic = create_info.scissor {
                dynamic_states.push(vk::DynamicState::SCISSOR);
            }

            let dynamic_state_info = vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_states).build();

            let depth_stencil_info = vk::PipelineDepthStencilStateCreateInfo::builder()
                .depth_test_enable(true)
                .depth_write_enable(true)
                .depth_compare_op(vk::CompareOp::LESS)
                .depth_bounds_test_enable(false)
                .stencil_test_enable(false)
                .build();

            let pipeline = vust.device.create_graphics_pipelines(
                vk::PipelineCache::null(),
                &[
                    vk::GraphicsPipelineCreateInfo::builder()
                        .stages(&shader_stages)
                        .vertex_input_state(&vertex_input_info)
                        .input_assembly_state(&input_assembly_info)
                        .viewport_state(&viewport_state_info)
                        .rasterization_state(&rasterizer_info)
                        .multisample_state(&multisample_info)
                        .depth_stencil_state(&depth_stencil_info)
                        .color_blend_state(&color_blend_info)
                        .dynamic_state(&dynamic_state_info)
                        .layout(pipeline_layout)
                        .render_pass(vust.renderpass)
                        .subpass(0)
                        .build()
                ],
                None
            ).unwrap()[0];

            let descriptor_pool_create_info = if let Some(descriptor_set_layout) = &create_info.descriptor_set_layout {
                let pool_sizes = descriptor_set_layout.bindings
                    .iter()
                    .map(|bindings| {
                        vk::DescriptorPoolSize::builder()
                            .ty(bindings.descriptor_type)
                            .descriptor_count(Vust::MAX_FRAMES_IN_FLIGHT as u32)
                            .build()
                    })
                    .collect::<Vec<_>>();

                Some((
                    vk::DescriptorPoolCreateInfo::builder()
                        .max_sets(Vust::MAX_FRAMES_IN_FLIGHT as u32)
                        .pool_sizes(&pool_sizes)
                        .build(),
                    pool_sizes
                ))
            } else {
                None
            };

            let write_descriptor_set_info = if let Some(descriptor_set_layout) = create_info.descriptor_set_layout {
                descriptor_set_layout.bindings.iter().enumerate().map(|(i, descriptor_set_binding)| {
                    let writes = [   
                        vk::WriteDescriptorSet::builder()
                            .dst_binding(i as u32)
                            .dst_array_element(0)
                            .descriptor_type(descriptor_set_binding.descriptor_type)
                            .build(); Vust::MAX_FRAMES_IN_FLIGHT
                    ];

                    writes
                }).collect::<Vec<_>>()
            } else {
                vec![]
            };

            GraphicsPipeline {
                descriptor_pool_create_info,
                descriptor_set_layout,
                write_descriptor_set_info,
                pipeline_layout,
                pipeline
            }
        }
    }

    pub fn create_descriptor(&self, vust: &Vust) -> Option<Descriptor> {
        unsafe {
            let descriptor_pool = vust.device.create_descriptor_pool(
                &self.descriptor_pool_create_info.as_ref()?.0,
                None
            ).ok()?;

            let descriptor_set: [vk::DescriptorSet; Vust::MAX_FRAMES_IN_FLIGHT] = vust.device.allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::builder()
                    .descriptor_pool(descriptor_pool)
                    .set_layouts(&[self.descriptor_set_layout; Vust::MAX_FRAMES_IN_FLIGHT])
            ).unwrap().try_into().unwrap();
            
            let write_descriptor_set_info = self.write_descriptor_set_info.clone().into_iter().map(|mut write_descriptor_infos| {
                for i in 0..Vust::MAX_FRAMES_IN_FLIGHT {
                    write_descriptor_infos[i].dst_set = descriptor_set[i];
                }

                write_descriptor_infos
            }).collect::<Vec<_>>();

            Some(Descriptor {
                descriptor_pool,
                descriptor_set,
                write_descriptor_set_info
            })
        }
    }

    pub fn handle(&self) -> vk::Pipeline {
        self.pipeline
    }

    pub fn pipeline_layout(&self) -> vk::PipelineLayout {
        self.pipeline_layout
    }
}

pub struct GraphicsPipelineCreateInfo {
    pub name: String,
    pub vertex_bin: Vec<u8>,
    pub fragment_bin: Vec<u8>,
    pub vertex_binding_descriptions: Vec<VertexInputBindingDescription>,
    pub vertex_attribute_descriptions: Vec<VertexInputAttributeDescription>,
    pub topology: vk::PrimitiveTopology,
    pub viewport: Viewport,
    pub scissor: Scissor,
    pub polygon_mode: vk::PolygonMode,
    pub cull_mode: CullMode,
    pub descriptor_set_layout: Option<DescriptorSetLayout>
}

#[derive(Debug, Clone)]
pub enum Viewport {
    Dynamic,
    Static {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        min_depth: f32,
        max_depth: f32
    }
}

#[derive(Debug, Clone)]
pub enum Scissor {
    Dynamic,
    Static {
        x: i32,
        y: i32,
        width: u32,
        height: u32
    }
}

pub enum CullMode {
    Clockwise,
    AntiClockwise,
    None
}

pub struct DescriptorSetLayout {
    /// Make sure to order the bindings correctly, as the index of the DescriptorSetBinding in the vector is used as the binding index for the descriptor binding.
    /// 
    /// e.g 
    /// ``` rust
    /// vec![
    ///     camera_binding, 
    ///     model_binding
    /// ] = [
    ///     vk::DescriptorSetLayoutBinding {
    ///         binding: 0,
    ///         camera_binding info.. 
    ///     },
    ///     vk::DescriptorSetLayoutBinding {
    ///         binding: 1,
    ///         model_binding info.. 
    ///     }
    /// ];
    /// ```
    pub bindings: Vec<DescriptorSetBinding>
}

pub struct DescriptorSetBinding {
    pub descriptor_type: vk::DescriptorType,
    pub stage_flags: vk::ShaderStageFlags
}
