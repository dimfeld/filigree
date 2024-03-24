import { client, type ModelDefinition } from "filigree-web";
import { z } from "zod";
import { ObjectPermission } from "../model_types.js";
import {
	ReportSectionCreatePayloadSchema,
	ReportSectionSchema,
	ReportSectionUpdatePayloadSchema,
} from "./report_section.js";

export const ReportSchema = z.object({
	id: z.string(),
	organization_id: z.string(),
	updated_at: z.string().datetime(),
	created_at: z.string().datetime(),
	title: z.string(),
	description: z.string().optional(),
	ui: z.any(),
	_permission: ObjectPermission,
});

export type Report = z.infer<typeof ReportSchema>;

export const ReportCreatePayloadSchema = z.object({
	id: z.string().optional(),
	title: z.string(),
	description: z.string().optional(),
	ui: z.any(),
	report_sections: ReportSectionCreatePayloadSchema.array().optional(),
});

export type ReportCreatePayload = z.infer<typeof ReportCreatePayloadSchema>;

export const ReportPopulatedGetAndCreateResultSchema = z.object({
	id: z.string(),
	organization_id: z.string(),
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
	id: z.string(),
	organization_id: z.string(),
	updated_at: z.string().datetime(),
	created_at: z.string().datetime(),
	title: z.string(),
	description: z.string().optional(),
	ui: z.any(),
	report_section_ids: z.string().array(),
	_permission: ObjectPermission,
});

export type ReportPopulatedList = z.infer<typeof ReportPopulatedListSchema>;

export const ReportUpdatePayloadSchema = z.object({
	id: z.string().optional(),
	title: z.string(),
	description: z.string().optional(),
	ui: z.any().optional(),
	report_sections: ReportSectionUpdatePayloadSchema.array().optional(),
});

export type ReportUpdatePayload = z.infer<typeof ReportUpdatePayloadSchema>;

export const baseUrl = "reports";
export const urlWithId = (id: string) => `${baseUrl}/${id}`;

export const urls = {
	create: baseUrl,
	list: baseUrl,
	get: urlWithId,
	update: urlWithId,
	delete: urlWithId,
};

export const ReportModel: ModelDefinition<typeof ReportSchema> = {
	name: "Report",
	plural: "Reports",
	baseUrl,
	urls,
	schema: ReportSchema,
	createSchema: ReportCreatePayloadSchema,
	updateSchema: ReportUpdatePayloadSchema,
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
