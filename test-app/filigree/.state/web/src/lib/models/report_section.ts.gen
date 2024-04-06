import { client, type ModelDefinition } from "filigree-web";
import { z } from "zod";
import { ObjectPermission } from "../model_types.js";

export type ReportSectionId = string;

export const ReportSectionSchema = z.object({
	id: z.string(),
	organization_id: z.string(),
	updated_at: z.string().datetime(),
	created_at: z.string().datetime(),
	name: z.string(),
	viz: z.string(),
	options: z.any(),
	report_id: z.string().uuid(),
	_permission: ObjectPermission,
});

export type ReportSection = z.infer<typeof ReportSectionSchema>;
export const ReportSectionListResultSchema = ReportSectionSchema;
export type ReportSectionListResult = ReportSection;
export const ReportSectionPopulatedGetResultSchema = ReportSectionSchema;
export type ReportSectionPopulatedGetResult = ReportSection;
export const ReportSectionPopulatedListResultSchema = ReportSectionSchema;
export type ReportSectionPopulatedListResult = ReportSection;
export const ReportSectionCreateResultSchema = ReportSectionSchema;
export type ReportSectionCreateResult = ReportSection;

export const ReportSectionCreatePayloadAndUpdatePayloadSchema = z.object({
	id: z.string().optional(),
	name: z.string(),
	viz: z.string(),
	options: z.any(),
	report_id: z.string().uuid(),
});

export type ReportSectionCreatePayloadAndUpdatePayload = z.infer<
	typeof ReportSectionCreatePayloadAndUpdatePayloadSchema
>;
export const ReportSectionCreatePayloadSchema =
	ReportSectionCreatePayloadAndUpdatePayloadSchema;
export type ReportSectionCreatePayload =
	ReportSectionCreatePayloadAndUpdatePayload;
export const ReportSectionUpdatePayloadSchema =
	ReportSectionCreatePayloadAndUpdatePayloadSchema;
export type ReportSectionUpdatePayload =
	ReportSectionCreatePayloadAndUpdatePayload;

export const baseUrl = "report_sections";
export const urlWithId = (id: string) => `${baseUrl}/${id}`;

export const urls = {
	create: baseUrl,
	list: baseUrl,
	get: urlWithId,
	update: urlWithId,
	delete: urlWithId,
};

export const ReportSectionModel: ModelDefinition<typeof ReportSectionSchema> = {
	name: "ReportSection",
	plural: "ReportSections",
	baseUrl,
	urls,
	schema: ReportSectionSchema,
	createSchema: ReportSectionCreatePayloadSchema,
	updateSchema: ReportSectionUpdatePayloadSchema,
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
			name: "name",
			type: "text",
			label: "Name",
			constraints: {
				required: true,
			},
		},
		{
			name: "viz",
			type: "text",
			label: "Viz",
			constraints: {
				required: true,
			},
		},
		{
			name: "options",
			type: "object",
			label: "Options",
			constraints: {
				required: true,
			},
		},
		{
			name: "report_id",
			type: "uuid",
			label: "Report Id",
			constraints: {
				required: true,
			},
		},
	],
};
