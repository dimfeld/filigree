import type { ModelDefinition } from "filigree-web";
import { z } from "zod";
import { ObjectPermission } from "../model_types.js";

export const UserSchema = z.object({
	id: z.string().uuid(),
	organization_id: z.string().uuid().optional(),
	updated_at: z.string().datetime(),
	created_at: z.string().datetime(),
	name: z.string(),
	email: z.string().optional(),
	avatar_url: z.string().optional(),
	_permission: ObjectPermission,
});

export type User = z.infer<typeof UserSchema>;
export const UserPopulatedGetSchema = UserSchema;
export type UserPopulatedGet = User;
export const UserPopulatedListSchema = UserSchema;
export type UserPopulatedList = User;
export const UserCreateResultSchema = UserSchema;
export type UserCreateResult = User;

export const UserCreatePayloadAndUpdatePayloadSchema = z.object({
	id: z.string().uuid().optional(),
	name: z.string(),
	email: z.string().optional(),
	avatar_url: z.string().optional(),
});

export type UserCreatePayloadAndUpdatePayload = z.infer<
	typeof UserCreatePayloadAndUpdatePayloadSchema
>;
export const UserCreatePayloadSchema = UserCreatePayloadAndUpdatePayloadSchema;
export type UserCreatePayload = UserCreatePayloadAndUpdatePayload;
export const UserUpdatePayloadSchema = UserCreatePayloadAndUpdatePayloadSchema;
export type UserUpdatePayload = UserCreatePayloadAndUpdatePayload;

export const UserModel: ModelDefinition<typeof UserSchema> = {
	name: "User",
	plural: "Users",
	url: "users",
	schema: UserSchema,
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
				required: false,
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
			name: "password_hash",
			type: "text",
			label: "Password Hash",
			constraints: {
				required: false,
			},
		},
		{
			name: "email",
			type: "text",
			label: "Email",
			constraints: {
				required: false,
			},
		},
		{
			name: "avatar_url",
			type: "text",
			label: "Avatar Url",
			constraints: {
				required: false,
			},
		},
	],
};
