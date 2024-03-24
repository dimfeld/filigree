import { client, type ModelDefinition } from "filigree-web";
import { z } from "zod";
import { ObjectPermission } from "../model_types.js";

export const RoleSchema = z.object({
	id: z.string(),
	organization_id: z.string(),
	updated_at: z.string().datetime(),
	created_at: z.string().datetime(),
	name: z.string(),
	description: z.string().optional(),
	_permission: ObjectPermission,
});

export type Role = z.infer<typeof RoleSchema>;
export const RolePopulatedGetSchema = RoleSchema;
export type RolePopulatedGet = Role;
export const RolePopulatedListSchema = RoleSchema;
export type RolePopulatedList = Role;
export const RoleCreateResultSchema = RoleSchema;
export type RoleCreateResult = Role;

export const RoleCreatePayloadAndUpdatePayloadSchema = z.object({
	id: z.string().optional(),
	name: z.string(),
	description: z.string().optional(),
});

export type RoleCreatePayloadAndUpdatePayload = z.infer<
	typeof RoleCreatePayloadAndUpdatePayloadSchema
>;
export const RoleCreatePayloadSchema = RoleCreatePayloadAndUpdatePayloadSchema;
export type RoleCreatePayload = RoleCreatePayloadAndUpdatePayload;
export const RoleUpdatePayloadSchema = RoleCreatePayloadAndUpdatePayloadSchema;
export type RoleUpdatePayload = RoleCreatePayloadAndUpdatePayload;

export const baseUrl = "roles";
export const urlWithId = (id: string) => `${baseUrl}/${id}`;

export const urls = {
	create: baseUrl,
	list: baseUrl,
	get: urlWithId,
	update: urlWithId,
	delete: urlWithId,
};

export const RoleModel: ModelDefinition<typeof RoleSchema> = {
	name: "Role",
	plural: "Roles",
	baseUrl,
	urls,
	schema: RoleSchema,
	createSchema: RoleCreatePayloadSchema,
	updateSchema: RoleUpdatePayloadSchema,
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
			name: "description",
			type: "text",
			label: "Description",
			constraints: {
				required: false,
			},
		},
	],
};
