import type { ModelDefinition } from "filigree-web";
import { z } from "zod";
import { ObjectPermission } from "../model_types.js";
import {
	ReportSectionSchema,
	ReportSectionUpdatePayloadSchema,
	ReportSectionCreatePayloadSchema,
} from "./report_section.js";

export const ReportSchema = z.object({
	id: z.string().uuid(),
	organization_id: z.string().uuid(),
	updated_at: z.string().datetime(),
	created_at: z.string().datetime(),
	title: z.string(),
	description: z.string().optional(),
	ui: z.any(),
	_permission: ObjectPermission,
});

export type Report = z.infer<typeof ReportSchema>;

export const ReportCreatePayloadSchema = z.object({
	id: z.string().uuid().optional(),
	title: z.string(),
	description: z.string().optional(),
	ui: z.any(),
	report_sections: ReportSectionCreatePayloadSchema.array().optional(),
});

export type ReportCreatePayload = z.infer<typeof ReportCreatePayloadSchema>;

export const ReportPopulatedGetAndCreateResultSchema = z.object({
	id: z.string().uuid(),
	organization_id: z.string().uuid(),
	updated_at: z.string().datetime(),
	created_at: z.string().datetime(),
	title: z.string(),
	description: z.string().optional(),
	ui: z.any(),
	report_sections: ReportSectionSchema.array(),
	_permission: ObjectPermission,
});

export type ReportPopulatedGetAndCreateResult = z.infer<
	typeof ReportPopulatedGetAndCreateResultSchema
>;
export const ReportPopulatedGetSchema = ReportPopulatedGetAndCreateResultSchema;
export type ReportPopulatedGet = ReportPopulatedGetAndCreateResult;
export const ReportCreateResultSchema = ReportPopulatedGetAndCreateResultSchema;
export type ReportCreateResult = ReportPopulatedGetAndCreateResult;

export const ReportPopulatedListSchema = z.object({
	id: z.string().uuid(),
	organization_id: z.string().uuid(),
	updated_at: z.string().datetime(),
	created_at: z.string().datetime(),
	title: z.string(),
	description: z.string().optional(),
	ui: z.any(),
	report_section_ids: z.string().uuid().array(),
	_permission: ObjectPermission,
});

export type ReportPopulatedList = z.infer<typeof ReportPopulatedListSchema>;

export const ReportUpdatePayloadSchema = z.object({
	id: z.string().uuid().optional(),
	title: z.string(),
	description: z.string().optional(),
	ui: z.any().optional(),
	report_sections: ReportSectionUpdatePayloadSchema.array().optional(),
});

export type ReportUpdatePayload = z.infer<typeof ReportUpdatePayloadSchema>;

export const ReportModel: ModelDefinition<typeof ReportSchema> = {
	name: "Report",
	plural: "Reports",
	url: "reports",
	schema: ReportSchema,
	fields: [
		{
			name: "id",
			type: "uuid",
			label: "Id",
			constraints: {
				required: true,
			},
		},
		{
			name: "organization_id",
			type: "uuid",
			label: "Organization Id",
			constraints: {
				required: true,
			},
		},
		{
			name: "updated_at",
			type: "date-time",
			label: "Updated At",
			constraints: {
				required: true,
			},
		},
		{
			name: "created_at",
			type: "date-time",
			label: "Created At",
			constraints: {
				required: true,
			},
		},
		{
			name: "title",
			type: "text",
			label: "Title",
			constraints: {
				required: true,
			},
		},
		{
			name: "description",
			type: "text",
			label: "Description",
			constraints: {
				required: false,
			},
		},
		{
			name: "ui",
			type: "object",
			label: "Ui",
			constraints: {
				required: true,
			},
		},
	],
};
