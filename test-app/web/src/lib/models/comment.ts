import { client, type ModelDefinition } from "filigree-web";
import { z } from "zod";
import { ObjectPermission } from "../model_types.js";

export type CommentId = string;

export const CommentSchema = z.object({
	id: z.string(),
	organization_id: z.string(),
	updated_at: z.string().datetime(),
	created_at: z.string().datetime(),
	body: z.string(),
	post_id: z.string().uuid(),
	_permission: ObjectPermission,
});

export type Comment = z.infer<typeof CommentSchema>;
export const CommentListResultSchema = CommentSchema;
export type CommentListResult = Comment;
export const CommentPopulatedGetResultSchema = CommentSchema;
export type CommentPopulatedGetResult = Comment;
export const CommentPopulatedListResultSchema = CommentSchema;
export type CommentPopulatedListResult = Comment;
export const CommentCreateResultSchema = CommentSchema;
export type CommentCreateResult = Comment;

export const CommentCreatePayloadAndUpdatePayloadSchema = z.object({
	id: z.string().optional(),
	body: z.string(),
	post_id: z.string().uuid(),
});

export type CommentCreatePayloadAndUpdatePayload = z.infer<
	typeof CommentCreatePayloadAndUpdatePayloadSchema
>;
export const CommentCreatePayloadSchema =
	CommentCreatePayloadAndUpdatePayloadSchema;
export type CommentCreatePayload = CommentCreatePayloadAndUpdatePayload;
export const CommentUpdatePayloadSchema =
	CommentCreatePayloadAndUpdatePayloadSchema;
export type CommentUpdatePayload = CommentCreatePayloadAndUpdatePayload;

export const baseUrl = "comments";
export const urlWithId = (id: string) => `${baseUrl}/${id}`;

export const urls = {
	create: baseUrl,
	list: baseUrl,
	get: urlWithId,
	update: urlWithId,
	delete: urlWithId,
};

export const CommentModel: ModelDefinition<typeof CommentSchema> = {
	name: "Comment",
	plural: "Comments",
	baseUrl,
	urls,
	schema: CommentSchema,
	createSchema: CommentCreatePayloadSchema,
	updateSchema: CommentUpdatePayloadSchema,
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
			name: "body",
			type: "text",
			label: "Body",
			constraints: {
				required: true,
			},
		},
		{
			name: "post_id",
			type: "uuid",
			label: "Post Id",
			constraints: {
				required: true,
			},
		},
	],
};
