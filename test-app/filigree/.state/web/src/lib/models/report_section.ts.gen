import type { ModelDefinition } from "filigree-web";
import { z } from "zod";
import { ObjectPermission } from "../model_types.js";

export const ReportSectionSchema = z.object({
	id: z.string().uuid(),
	organization_id: z.string().uuid(),
	updated_at: z.string().datetime(),
	created_at: z.string().datetime(),
	name: z.string(),
	viz: z.string(),
	options: z.any(),
	report_id: z.string().uuid(),
	_permission: ObjectPermission,
});

export type ReportSection = z.infer<typeof ReportSectionSchema>;
export const ReportSectionPopulatedGetSchema = ReportSectionSchema;
export type ReportSectionPopulatedGet = ReportSection;
export const ReportSectionPopulatedListSchema = ReportSectionSchema;
export type ReportSectionPopulatedList = ReportSection;
export const ReportSectionCreateResultSchema = ReportSectionSchema;
export type ReportSectionCreateResult = ReportSection;

export const ReportSectionCreatePayloadAndUpdatePayloadSchema = z.object({
	id: z.string().uuid().optional(),
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

export const ReportSectionModel: ModelDefinition<typeof ReportSectionSchema> = {
	name: "ReportSection",
	plural: "ReportSections",
	url: "report_sections",
	schema: ReportSectionSchema,
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
