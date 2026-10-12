// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint, type=warning, deprecated_member_use, deprecated_member_use_from_same_package
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'types.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$UiEvent {





@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is UiEvent);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'UiEvent()';
}


}

/// @nodoc
class $UiEventCopyWith<$Res>  {
$UiEventCopyWith(UiEvent _, $Res Function(UiEvent) __);
}


/// Adds pattern-matching-related methods to [UiEvent].
extension UiEventPatterns on UiEvent {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( UiEvent_Progress value)?  progress,TResult Function( UiEvent_TaskFinished value)?  taskFinished,TResult Function( UiEvent_AccountsChanged value)?  accountsChanged,TResult Function( UiEvent_GameStarted value)?  gameStarted,TResult Function( UiEvent_GameLog value)?  gameLog,TResult Function( UiEvent_GameCrashed value)?  gameCrashed,TResult Function( UiEvent_GameExited value)?  gameExited,required TResult orElse(),}){
final _that = this;
switch (_that) {
case UiEvent_Progress() when progress != null:
return progress(_that);case UiEvent_TaskFinished() when taskFinished != null:
return taskFinished(_that);case UiEvent_AccountsChanged() when accountsChanged != null:
return accountsChanged(_that);case UiEvent_GameStarted() when gameStarted != null:
return gameStarted(_that);case UiEvent_GameLog() when gameLog != null:
return gameLog(_that);case UiEvent_GameCrashed() when gameCrashed != null:
return gameCrashed(_that);case UiEvent_GameExited() when gameExited != null:
return gameExited(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( UiEvent_Progress value)  progress,required TResult Function( UiEvent_TaskFinished value)  taskFinished,required TResult Function( UiEvent_AccountsChanged value)  accountsChanged,required TResult Function( UiEvent_GameStarted value)  gameStarted,required TResult Function( UiEvent_GameLog value)  gameLog,required TResult Function( UiEvent_GameCrashed value)  gameCrashed,required TResult Function( UiEvent_GameExited value)  gameExited,}){
final _that = this;
switch (_that) {
case UiEvent_Progress():
return progress(_that);case UiEvent_TaskFinished():
return taskFinished(_that);case UiEvent_AccountsChanged():
return accountsChanged(_that);case UiEvent_GameStarted():
return gameStarted(_that);case UiEvent_GameLog():
return gameLog(_that);case UiEvent_GameCrashed():
return gameCrashed(_that);case UiEvent_GameExited():
return gameExited(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( UiEvent_Progress value)?  progress,TResult? Function( UiEvent_TaskFinished value)?  taskFinished,TResult? Function( UiEvent_AccountsChanged value)?  accountsChanged,TResult? Function( UiEvent_GameStarted value)?  gameStarted,TResult? Function( UiEvent_GameLog value)?  gameLog,TResult? Function( UiEvent_GameCrashed value)?  gameCrashed,TResult? Function( UiEvent_GameExited value)?  gameExited,}){
final _that = this;
switch (_that) {
case UiEvent_Progress() when progress != null:
return progress(_that);case UiEvent_TaskFinished() when taskFinished != null:
return taskFinished(_that);case UiEvent_AccountsChanged() when accountsChanged != null:
return accountsChanged(_that);case UiEvent_GameStarted() when gameStarted != null:
return gameStarted(_that);case UiEvent_GameLog() when gameLog != null:
return gameLog(_that);case UiEvent_GameCrashed() when gameCrashed != null:
return gameCrashed(_that);case UiEvent_GameExited() when gameExited != null:
return gameExited(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( BigInt taskId,  String stage,  BigInt done,  BigInt total,  bool bytes)?  progress,TResult Function( BigInt taskId,  bool ok,  String? message)?  taskFinished,TResult Function()?  accountsChanged,TResult Function( BigInt gameId,  String instanceId,  int? pid)?  gameStarted,TResult Function( BigInt gameId,  List<LogLineView> lines)?  gameLog,TResult Function( BigInt gameId,  String instanceId,  String? crashReport)?  gameCrashed,TResult Function( BigInt gameId,  String instanceId,  int? code,  BigInt secondsPlayed)?  gameExited,required TResult orElse(),}) {final _that = this;
switch (_that) {
case UiEvent_Progress() when progress != null:
return progress(_that.taskId,_that.stage,_that.done,_that.total,_that.bytes);case UiEvent_TaskFinished() when taskFinished != null:
return taskFinished(_that.taskId,_that.ok,_that.message);case UiEvent_AccountsChanged() when accountsChanged != null:
return accountsChanged();case UiEvent_GameStarted() when gameStarted != null:
return gameStarted(_that.gameId,_that.instanceId,_that.pid);case UiEvent_GameLog() when gameLog != null:
return gameLog(_that.gameId,_that.lines);case UiEvent_GameCrashed() when gameCrashed != null:
return gameCrashed(_that.gameId,_that.instanceId,_that.crashReport);case UiEvent_GameExited() when gameExited != null:
return gameExited(_that.gameId,_that.instanceId,_that.code,_that.secondsPlayed);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( BigInt taskId,  String stage,  BigInt done,  BigInt total,  bool bytes)  progress,required TResult Function( BigInt taskId,  bool ok,  String? message)  taskFinished,required TResult Function()  accountsChanged,required TResult Function( BigInt gameId,  String instanceId,  int? pid)  gameStarted,required TResult Function( BigInt gameId,  List<LogLineView> lines)  gameLog,required TResult Function( BigInt gameId,  String instanceId,  String? crashReport)  gameCrashed,required TResult Function( BigInt gameId,  String instanceId,  int? code,  BigInt secondsPlayed)  gameExited,}) {final _that = this;
switch (_that) {
case UiEvent_Progress():
return progress(_that.taskId,_that.stage,_that.done,_that.total,_that.bytes);case UiEvent_TaskFinished():
return taskFinished(_that.taskId,_that.ok,_that.message);case UiEvent_AccountsChanged():
return accountsChanged();case UiEvent_GameStarted():
return gameStarted(_that.gameId,_that.instanceId,_that.pid);case UiEvent_GameLog():
return gameLog(_that.gameId,_that.lines);case UiEvent_GameCrashed():
return gameCrashed(_that.gameId,_that.instanceId,_that.crashReport);case UiEvent_GameExited():
return gameExited(_that.gameId,_that.instanceId,_that.code,_that.secondsPlayed);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( BigInt taskId,  String stage,  BigInt done,  BigInt total,  bool bytes)?  progress,TResult? Function( BigInt taskId,  bool ok,  String? message)?  taskFinished,TResult? Function()?  accountsChanged,TResult? Function( BigInt gameId,  String instanceId,  int? pid)?  gameStarted,TResult? Function( BigInt gameId,  List<LogLineView> lines)?  gameLog,TResult? Function( BigInt gameId,  String instanceId,  String? crashReport)?  gameCrashed,TResult? Function( BigInt gameId,  String instanceId,  int? code,  BigInt secondsPlayed)?  gameExited,}) {final _that = this;
switch (_that) {
case UiEvent_Progress() when progress != null:
return progress(_that.taskId,_that.stage,_that.done,_that.total,_that.bytes);case UiEvent_TaskFinished() when taskFinished != null:
return taskFinished(_that.taskId,_that.ok,_that.message);case UiEvent_AccountsChanged() when accountsChanged != null:
return accountsChanged();case UiEvent_GameStarted() when gameStarted != null:
return gameStarted(_that.gameId,_that.instanceId,_that.pid);case UiEvent_GameLog() when gameLog != null:
return gameLog(_that.gameId,_that.lines);case UiEvent_GameCrashed() when gameCrashed != null:
return gameCrashed(_that.gameId,_that.instanceId,_that.crashReport);case UiEvent_GameExited() when gameExited != null:
return gameExited(_that.gameId,_that.instanceId,_that.code,_that.secondsPlayed);case _:
  return null;

}
}

}

/// @nodoc


class UiEvent_Progress extends UiEvent {
  const UiEvent_Progress({required this.taskId, required this.stage, required this.done, required this.total, required this.bytes}): super._();
  

 final  BigInt taskId;
 final  String stage;
 final  BigInt done;
 final  BigInt total;
 final  bool bytes;

/// Create a copy of UiEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiEvent_ProgressCopyWith<UiEvent_Progress> get copyWith => _$UiEvent_ProgressCopyWithImpl<UiEvent_Progress>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is UiEvent_Progress&&(identical(other.taskId, taskId) || other.taskId == taskId)&&(identical(other.stage, stage) || other.stage == stage)&&(identical(other.done, done) || other.done == done)&&(identical(other.total, total) || other.total == total)&&(identical(other.bytes, bytes) || other.bytes == bytes));
}


@override
int get hashCode {
    return Object.hash(runtimeType,taskId,stage,done,total,bytes);
}

@override
String toString() {
    return 'UiEvent.progress(taskId: $taskId, stage: $stage, done: $done, total: $total, bytes: $bytes)';
}


}

/// @nodoc
abstract mixin class $UiEvent_ProgressCopyWith<$Res> implements $UiEventCopyWith<$Res> {
  factory $UiEvent_ProgressCopyWith(UiEvent_Progress value, $Res Function(UiEvent_Progress) _then) = _$UiEvent_ProgressCopyWithImpl;
@useResult
$Res call({
 BigInt taskId, String stage, BigInt done, BigInt total, bool bytes
});




}
/// @nodoc
class _$UiEvent_ProgressCopyWithImpl<$Res>
    implements $UiEvent_ProgressCopyWith<$Res> {
  _$UiEvent_ProgressCopyWithImpl(this._self, this._then);

  final UiEvent_Progress _self;
  final $Res Function(UiEvent_Progress) _then;

/// Create a copy of UiEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? taskId = null,Object? stage = null,Object? done = null,Object? total = null,Object? bytes = null,}) {
  return _then(UiEvent_Progress(
taskId: null == taskId ? _self.taskId : taskId // ignore: cast_nullable_to_non_nullable
as BigInt,stage: null == stage ? _self.stage : stage // ignore: cast_nullable_to_non_nullable
as String,done: null == done ? _self.done : done // ignore: cast_nullable_to_non_nullable
as BigInt,total: null == total ? _self.total : total // ignore: cast_nullable_to_non_nullable
as BigInt,bytes: null == bytes ? _self.bytes : bytes // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}


}

/// @nodoc


class UiEvent_TaskFinished extends UiEvent {
  const UiEvent_TaskFinished({required this.taskId, required this.ok, this.message}): super._();
  

 final  BigInt taskId;
 final  bool ok;
 final  String? message;

/// Create a copy of UiEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiEvent_TaskFinishedCopyWith<UiEvent_TaskFinished> get copyWith => _$UiEvent_TaskFinishedCopyWithImpl<UiEvent_TaskFinished>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is UiEvent_TaskFinished&&(identical(other.taskId, taskId) || other.taskId == taskId)&&(identical(other.ok, ok) || other.ok == ok)&&(identical(other.message, message) || other.message == message));
}


@override
int get hashCode {
    return Object.hash(runtimeType,taskId,ok,message);
}

@override
String toString() {
    return 'UiEvent.taskFinished(taskId: $taskId, ok: $ok, message: $message)';
}


}

/// @nodoc
abstract mixin class $UiEvent_TaskFinishedCopyWith<$Res> implements $UiEventCopyWith<$Res> {
  factory $UiEvent_TaskFinishedCopyWith(UiEvent_TaskFinished value, $Res Function(UiEvent_TaskFinished) _then) = _$UiEvent_TaskFinishedCopyWithImpl;
@useResult
$Res call({
 BigInt taskId, bool ok, String? message
});




}
/// @nodoc
class _$UiEvent_TaskFinishedCopyWithImpl<$Res>
    implements $UiEvent_TaskFinishedCopyWith<$Res> {
  _$UiEvent_TaskFinishedCopyWithImpl(this._self, this._then);

  final UiEvent_TaskFinished _self;
  final $Res Function(UiEvent_TaskFinished) _then;

/// Create a copy of UiEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? taskId = null,Object? ok = null,Object? message = freezed,}) {
  return _then(UiEvent_TaskFinished(
taskId: null == taskId ? _self.taskId : taskId // ignore: cast_nullable_to_non_nullable
as BigInt,ok: null == ok ? _self.ok : ok // ignore: cast_nullable_to_non_nullable
as bool,message: freezed == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String?,
  ));
}


}

/// @nodoc


class UiEvent_AccountsChanged extends UiEvent {
  const UiEvent_AccountsChanged(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is UiEvent_AccountsChanged);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'UiEvent.accountsChanged()';
}


}




/// @nodoc


class UiEvent_GameStarted extends UiEvent {
  const UiEvent_GameStarted({required this.gameId, required this.instanceId, this.pid}): super._();
  

 final  BigInt gameId;
 final  String instanceId;
 final  int? pid;

/// Create a copy of UiEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiEvent_GameStartedCopyWith<UiEvent_GameStarted> get copyWith => _$UiEvent_GameStartedCopyWithImpl<UiEvent_GameStarted>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is UiEvent_GameStarted&&(identical(other.gameId, gameId) || other.gameId == gameId)&&(identical(other.instanceId, instanceId) || other.instanceId == instanceId)&&(identical(other.pid, pid) || other.pid == pid));
}


@override
int get hashCode {
    return Object.hash(runtimeType,gameId,instanceId,pid);
}

@override
String toString() {
    return 'UiEvent.gameStarted(gameId: $gameId, instanceId: $instanceId, pid: $pid)';
}


}

/// @nodoc
abstract mixin class $UiEvent_GameStartedCopyWith<$Res> implements $UiEventCopyWith<$Res> {
  factory $UiEvent_GameStartedCopyWith(UiEvent_GameStarted value, $Res Function(UiEvent_GameStarted) _then) = _$UiEvent_GameStartedCopyWithImpl;
@useResult
$Res call({
 BigInt gameId, String instanceId, int? pid
});




}
/// @nodoc
class _$UiEvent_GameStartedCopyWithImpl<$Res>
    implements $UiEvent_GameStartedCopyWith<$Res> {
  _$UiEvent_GameStartedCopyWithImpl(this._self, this._then);

  final UiEvent_GameStarted _self;
  final $Res Function(UiEvent_GameStarted) _then;

/// Create a copy of UiEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? gameId = null,Object? instanceId = null,Object? pid = freezed,}) {
  return _then(UiEvent_GameStarted(
gameId: null == gameId ? _self.gameId : gameId // ignore: cast_nullable_to_non_nullable
as BigInt,instanceId: null == instanceId ? _self.instanceId : instanceId // ignore: cast_nullable_to_non_nullable
as String,pid: freezed == pid ? _self.pid : pid // ignore: cast_nullable_to_non_nullable
as int?,
  ));
}


}

/// @nodoc


class UiEvent_GameLog extends UiEvent {
  const UiEvent_GameLog({required this.gameId, required  List<LogLineView> lines}): _lines = lines,super._();
  

 final  BigInt gameId;
 final  List<LogLineView> _lines;
 List<LogLineView> get lines {
  if (_lines is EqualUnmodifiableListView) return _lines;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_lines);
}


/// Create a copy of UiEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiEvent_GameLogCopyWith<UiEvent_GameLog> get copyWith => _$UiEvent_GameLogCopyWithImpl<UiEvent_GameLog>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is UiEvent_GameLog&&(identical(other.gameId, gameId) || other.gameId == gameId)&&const DeepCollectionEquality().equals(other.lines, _lines));
}


@override
int get hashCode {
    return Object.hash(runtimeType,gameId,const DeepCollectionEquality().hash(_lines));
}

@override
String toString() {
    return 'UiEvent.gameLog(gameId: $gameId, lines: $lines)';
}


}

/// @nodoc
abstract mixin class $UiEvent_GameLogCopyWith<$Res> implements $UiEventCopyWith<$Res> {
  factory $UiEvent_GameLogCopyWith(UiEvent_GameLog value, $Res Function(UiEvent_GameLog) _then) = _$UiEvent_GameLogCopyWithImpl;
@useResult
$Res call({
 BigInt gameId, List<LogLineView> lines
});




}
/// @nodoc
class _$UiEvent_GameLogCopyWithImpl<$Res>
    implements $UiEvent_GameLogCopyWith<$Res> {
  _$UiEvent_GameLogCopyWithImpl(this._self, this._then);

  final UiEvent_GameLog _self;
  final $Res Function(UiEvent_GameLog) _then;

/// Create a copy of UiEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? gameId = null,Object? lines = null,}) {
  return _then(UiEvent_GameLog(
gameId: null == gameId ? _self.gameId : gameId // ignore: cast_nullable_to_non_nullable
as BigInt,lines: null == lines ? _self._lines : lines // ignore: cast_nullable_to_non_nullable
as List<LogLineView>,
  ));
}


}

/// @nodoc


class UiEvent_GameCrashed extends UiEvent {
  const UiEvent_GameCrashed({required this.gameId, required this.instanceId, this.crashReport}): super._();
  

 final  BigInt gameId;
 final  String instanceId;
 final  String? crashReport;

/// Create a copy of UiEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiEvent_GameCrashedCopyWith<UiEvent_GameCrashed> get copyWith => _$UiEvent_GameCrashedCopyWithImpl<UiEvent_GameCrashed>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is UiEvent_GameCrashed&&(identical(other.gameId, gameId) || other.gameId == gameId)&&(identical(other.instanceId, instanceId) || other.instanceId == instanceId)&&(identical(other.crashReport, crashReport) || other.crashReport == crashReport));
}


@override
int get hashCode {
    return Object.hash(runtimeType,gameId,instanceId,crashReport);
}

@override
String toString() {
    return 'UiEvent.gameCrashed(gameId: $gameId, instanceId: $instanceId, crashReport: $crashReport)';
}


}

/// @nodoc
abstract mixin class $UiEvent_GameCrashedCopyWith<$Res> implements $UiEventCopyWith<$Res> {
  factory $UiEvent_GameCrashedCopyWith(UiEvent_GameCrashed value, $Res Function(UiEvent_GameCrashed) _then) = _$UiEvent_GameCrashedCopyWithImpl;
@useResult
$Res call({
 BigInt gameId, String instanceId, String? crashReport
});




}
/// @nodoc
class _$UiEvent_GameCrashedCopyWithImpl<$Res>
    implements $UiEvent_GameCrashedCopyWith<$Res> {
  _$UiEvent_GameCrashedCopyWithImpl(this._self, this._then);

  final UiEvent_GameCrashed _self;
  final $Res Function(UiEvent_GameCrashed) _then;

/// Create a copy of UiEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? gameId = null,Object? instanceId = null,Object? crashReport = freezed,}) {
  return _then(UiEvent_GameCrashed(
gameId: null == gameId ? _self.gameId : gameId // ignore: cast_nullable_to_non_nullable
as BigInt,instanceId: null == instanceId ? _self.instanceId : instanceId // ignore: cast_nullable_to_non_nullable
as String,crashReport: freezed == crashReport ? _self.crashReport : crashReport // ignore: cast_nullable_to_non_nullable
as String?,
  ));
}


}

/// @nodoc


class UiEvent_GameExited extends UiEvent {
  const UiEvent_GameExited({required this.gameId, required this.instanceId, this.code, required this.secondsPlayed}): super._();
  

 final  BigInt gameId;
 final  String instanceId;
 final  int? code;
 final  BigInt secondsPlayed;

/// Create a copy of UiEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiEvent_GameExitedCopyWith<UiEvent_GameExited> get copyWith => _$UiEvent_GameExitedCopyWithImpl<UiEvent_GameExited>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is UiEvent_GameExited&&(identical(other.gameId, gameId) || other.gameId == gameId)&&(identical(other.instanceId, instanceId) || other.instanceId == instanceId)&&(identical(other.code, code) || other.code == code)&&(identical(other.secondsPlayed, secondsPlayed) || other.secondsPlayed == secondsPlayed));
}


@override
int get hashCode {
    return Object.hash(runtimeType,gameId,instanceId,code,secondsPlayed);
}

@override
String toString() {
    return 'UiEvent.gameExited(gameId: $gameId, instanceId: $instanceId, code: $code, secondsPlayed: $secondsPlayed)';
}


}

/// @nodoc
abstract mixin class $UiEvent_GameExitedCopyWith<$Res> implements $UiEventCopyWith<$Res> {
  factory $UiEvent_GameExitedCopyWith(UiEvent_GameExited value, $Res Function(UiEvent_GameExited) _then) = _$UiEvent_GameExitedCopyWithImpl;
@useResult
$Res call({
 BigInt gameId, String instanceId, int? code, BigInt secondsPlayed
});




}
/// @nodoc
class _$UiEvent_GameExitedCopyWithImpl<$Res>
    implements $UiEvent_GameExitedCopyWith<$Res> {
  _$UiEvent_GameExitedCopyWithImpl(this._self, this._then);

  final UiEvent_GameExited _self;
  final $Res Function(UiEvent_GameExited) _then;

/// Create a copy of UiEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? gameId = null,Object? instanceId = null,Object? code = freezed,Object? secondsPlayed = null,}) {
  return _then(UiEvent_GameExited(
gameId: null == gameId ? _self.gameId : gameId // ignore: cast_nullable_to_non_nullable
as BigInt,instanceId: null == instanceId ? _self.instanceId : instanceId // ignore: cast_nullable_to_non_nullable
as String,code: freezed == code ? _self.code : code // ignore: cast_nullable_to_non_nullable
as int?,secondsPlayed: null == secondsPlayed ? _self.secondsPlayed : secondsPlayed // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}

// dart format on
