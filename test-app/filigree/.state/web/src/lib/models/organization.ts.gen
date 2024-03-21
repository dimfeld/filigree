import type { ModelDefinition } from "filigree-web";
import { z } from "zod";
import { ObjectPermission } from "../model_types.js";

export const OrganizationSchema = z.object({
	id: z.string().uuid(),
	updated_at: z.string().datetime(),
	created_at: z.string().datetime(),
	name: z.string(),
	owner: z.string().uuid().optional(),
	default_role: z.string().uuid().optional(),
	_permission: ObjectPermission,
});

export type Organization = z.infer<typeof OrganizationSchema>;
export const OrganizationPopulatedGetSchema = OrganizationSchema;
export type OrganizationPopulatedGet = Organization;
export const OrganizationPopulatedListSchema = OrganizationSchema;
export type OrganizationPopulatedList = Organization;
export const OrganizationCreateResultSchema = OrganizationSchema;
export type OrganizationCreateResult = Organization;

export const OrganizationCreatePayloadAndUpdatePayloadSchema = z.object({
	id: z.string().uuid().optional(),
	name: z.string(),
	owner: z.string().uuid().optional(),
	default_role: z.string().uuid().optional(),
});

export type OrganizationCreatePayloadAndUpdatePayload = z.infer<
	typeof OrganizationCreatePayloadAndUpdatePayloadSchema
>;
export const OrganizationCreatePayloadSchema =
	OrganizationCreatePayloadAndUpdatePayloadSchema;
export type OrganizationCreatePayload =
	OrganizationCreatePayloadAndUpdatePayload;
export const OrganizationUpdatePayloadSchema =
	OrganizationCreatePayloadAndUpdatePayloadSchema;
export type OrganizationUpdatePayload =
	OrganizationCreatePayloadAndUpdatePayload;

export const OrganizationModel: ModelDefinition<typeof OrganizationSchema> = {
	name: "Organization",
	plural: "Organizations",
	url: "organizations",
	schema: OrganizationSchema,
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
			name: "owner",
			type: "uuid",
			label: "Owner",
			constraints: {
				required: false,
			},
		},
		{
			name: "default_role",
			type: "uuid",
			label: "Default Role",
			constraints: {
				required: false,
			},
		},
		{
			name: "active",
			type: "boolean",
			label: "Active",
			constraints: {
				required: true,
			},
		},
	],
};
